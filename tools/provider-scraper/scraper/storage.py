"""ZIP archive storage with ETag/content-hash caching for incremental crawls."""

import hashlib
import json
import logging
import re
import zipfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

logger = logging.getLogger(__name__)


@dataclass
class CacheEntry:
    """Cache entry for a crawled URL."""

    filename: str
    etag: str | None
    content_hash: str
    crawled_at: str


class DocsArchive:
    """Manages markdown docs in a ZIP archive with incremental caching."""

    def __init__(self, output_dir: Path):
        """Initialize with output directory. Creates docs.zip and cache.json."""
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)

        self.zip_path = self.output_dir / "docs.zip"
        self.cache_path = self.output_dir / "cache.json"

        self._cache: dict[str, CacheEntry] = self._load_cache()
        logger.debug(f"Initialized DocsArchive at {output_dir} with {len(self._cache)} cached entries")

    def _load_cache(self) -> dict[str, CacheEntry]:
        """Load cache.json, return empty dict if not exists."""
        if not self.cache_path.exists():
            logger.debug(f"No cache found at {self.cache_path}")
            return {}

        try:
            with open(self.cache_path, "r", encoding="utf-8") as f:
                data = json.load(f)

            # Convert dict to CacheEntry objects
            cache = {}
            for url, entry_dict in data.items():
                cache[url] = CacheEntry(**entry_dict)

            logger.debug(f"Loaded {len(cache)} entries from cache")
            return cache
        except (json.JSONDecodeError, TypeError, KeyError) as e:
            logger.error(f"Failed to load cache from {self.cache_path}: {e}")
            raise

    def _save_cache(self) -> None:
        """Save cache to cache.json."""
        try:
            # Convert CacheEntry objects to dicts
            data = {}
            for url, entry in self._cache.items():
                data[url] = {
                    "filename": entry.filename,
                    "etag": entry.etag,
                    "content_hash": entry.content_hash,
                    "crawled_at": entry.crawled_at,
                }

            with open(self.cache_path, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2)

            logger.debug(f"Saved {len(self._cache)} entries to cache")
        except (OSError, TypeError) as e:
            logger.error(f"Failed to save cache to {self.cache_path}: {e}")
            raise

    def _content_hash(self, content: str) -> str:
        """Compute SHA256 hash of content (first 16 chars)."""
        return hashlib.sha256(content.encode("utf-8")).hexdigest()[:16]

    def _safe_filename(self, url: str, topic: str) -> str:
        """Generate safe filename from URL/topic for ZIP entry.

        Args:
            url: The URL being crawled.
            topic: The topic/section identifier.

        Returns:
            Safe filename (alphanumeric, hyphens, underscores) with .md extension.
        """
        # Use topic if provided, otherwise extract from URL path
        if topic:
            base = topic
        else:
            # Extract path from URL (after domain)
            # Split by '/' and skip protocol and domain parts
            parts = url.rstrip("/").split("/")
            # parts[0] = 'https:', parts[1] = '', parts[2] = 'domain', parts[3:] = path
            if len(parts) > 3:
                # Use last path segment
                base = parts[-1]
            else:
                # No path, use 'index'
                base = "index"

        # Replace non-alphanumeric chars with hyphens
        safe = re.sub(r"[^a-zA-Z0-9_-]", "-", base)
        # Collapse multiple hyphens
        safe = re.sub(r"-+", "-", safe)
        # Strip leading/trailing hyphens
        safe = safe.strip("-")

        # Ensure we have a valid filename
        if not safe:
            safe = "page"

        return f"{safe}.md"

    def has_changed(self, url: str, etag: str | None, content: str) -> bool:
        """Check if content changed. Uses ETag if available, falls back to content hash.

        Args:
            url: The URL being checked.
            etag: Optional ETag from HTTP response.
            content: The content to check.

        Returns:
            True if content changed or URL is new, False if unchanged.
        """
        # New URL, always changed
        if url not in self._cache:
            logger.debug(f"URL not in cache (new): {url}")
            return True

        entry = self._cache[url]

        # If we have ETags from both sides, use them for comparison
        if etag and entry.etag:
            changed = etag != entry.etag
            logger.debug(f"ETag comparison for {url}: {'changed' if changed else 'unchanged'}")
            return changed

        # Fall back to content hash comparison
        new_hash = self._content_hash(content)
        changed = new_hash != entry.content_hash
        logger.debug(f"Content hash comparison for {url}: {'changed' if changed else 'unchanged'}")
        return changed

    def write(self, url: str, content: str, topic: str, etag: str | None = None) -> str:
        """Write markdown to ZIP, update cache. Returns filename.

        Args:
            url: The URL being crawled.
            content: The markdown content to store.
            topic: The topic/section identifier.
            etag: Optional ETag from HTTP response.

        Returns:
            The filename used in the ZIP archive.
        """
        filename = self._safe_filename(url, topic)
        content_hash = self._content_hash(content)
        crawled_at = datetime.now(timezone.utc).isoformat()

        # Update cache entry
        self._cache[url] = CacheEntry(
            filename=filename,
            etag=etag,
            content_hash=content_hash,
            crawled_at=crawled_at,
        )

        # Write to ZIP - we need to read all, update, write all since ZIP doesn't support in-place updates
        existing_files = {}
        if self.zip_path.exists():
            with zipfile.ZipFile(self.zip_path, "r") as zf:
                for name in zf.namelist():
                    existing_files[name] = zf.read(name)

        # Update or add new file
        existing_files[filename] = content.encode("utf-8")

        # Write all files to new ZIP
        with zipfile.ZipFile(self.zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
            for name, data in existing_files.items():
                zf.writestr(name, data)

        # Save cache
        self._save_cache()

        logger.info(f"Wrote {filename} to {self.zip_path} (URL: {url})")
        return filename

    def read(self, url: str) -> str | None:
        """Read markdown from ZIP by URL. Returns None if not found.

        Args:
            url: The URL to look up.

        Returns:
            The markdown content if found, None otherwise.
        """
        if url not in self._cache:
            logger.debug(f"URL not in cache: {url}")
            return None

        filename = self._cache[url].filename

        if not self.zip_path.exists():
            logger.warning(f"ZIP archive not found: {self.zip_path}")
            return None

        try:
            with zipfile.ZipFile(self.zip_path, "r") as zf:
                content = zf.read(filename).decode("utf-8")
            logger.debug(f"Read {filename} from {self.zip_path}")
            return content
        except KeyError:
            logger.warning(f"File {filename} not found in ZIP for URL: {url}")
            return None
        except (zipfile.BadZipFile, OSError) as e:
            logger.error(f"Failed to read {filename} from {self.zip_path}: {e}")
            raise
