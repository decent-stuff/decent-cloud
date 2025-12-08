"""ZIP archive storage with ETag/content-hash caching for incremental crawls.

Uses local directory during scraping for efficiency, zips only when finalized.
"""

import hashlib
import json
import logging
import re
import shutil
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
    """Manages markdown docs with local dir during scraping, ZIP on finalize."""

    def __init__(self, output_dir: Path):
        """Initialize with output directory. Unpacks existing ZIP to local docs/ dir."""
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)

        self.zip_path = self.output_dir / "docs.zip"
        self.cache_path = self.output_dir / "cache.json"
        self.docs_dir = self.output_dir / "docs"

        self._cache: dict[str, CacheEntry] = self._load_cache()

        # Unpack existing ZIP to local dir for incremental updates
        self._unpack_zip()

        logger.debug(f"Initialized DocsArchive at {output_dir} with {len(self._cache)} cached entries")

    def _unpack_zip(self) -> None:
        """Unpack existing ZIP to docs/ directory if it exists."""
        if not self.zip_path.exists():
            self.docs_dir.mkdir(parents=True, exist_ok=True)
            return

        # Clear existing docs dir and unpack fresh
        if self.docs_dir.exists():
            shutil.rmtree(self.docs_dir)
        self.docs_dir.mkdir(parents=True, exist_ok=True)

        try:
            with zipfile.ZipFile(self.zip_path, "r") as zf:
                zf.extractall(self.docs_dir)
            logger.debug(f"Unpacked {len(list(self.docs_dir.iterdir()))} files from {self.zip_path}")
        except zipfile.BadZipFile as e:
            logger.warning(f"Corrupted ZIP at {self.zip_path}, starting fresh: {e}")
            # Keep empty docs dir

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
        """Write markdown to local docs/ dir, update cache. Returns filename.

        Args:
            url: The URL being crawled.
            content: The markdown content to store.
            topic: The topic/section identifier.
            etag: Optional ETag from HTTP response.

        Returns:
            The filename used.
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

        # Write directly to local dir (fast!)
        file_path = self.docs_dir / filename
        file_path.write_text(content, encoding="utf-8")

        # Save cache
        self._save_cache()

        logger.info(f"Wrote {filename} to {self.docs_dir} (URL: {url})")
        return filename

    def finalize(self, keep_local: bool = False) -> None:
        """Create ZIP from docs/ dir and optionally clean up local files.

        Args:
            keep_local: If True, keep the docs/ directory for troubleshooting.
        """
        if not self.docs_dir.exists():
            logger.warning(f"No docs directory to finalize at {self.docs_dir}")
            return

        files = list(self.docs_dir.iterdir())
        if not files:
            logger.warning(f"No files in docs directory {self.docs_dir}")
            return

        # Create ZIP from docs/ contents
        with zipfile.ZipFile(self.zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
            for file_path in files:
                if file_path.is_file():
                    zf.write(file_path, file_path.name)

        logger.info(f"Created {self.zip_path} with {len(files)} files")

        # Clean up local dir unless keep_local
        if not keep_local:
            shutil.rmtree(self.docs_dir)
            logger.debug(f"Removed local docs directory {self.docs_dir}")

    def read(self, url: str) -> str | None:
        """Read markdown by URL. Tries local docs/ dir first, then ZIP.

        Args:
            url: The URL to look up.

        Returns:
            The markdown content if found, None otherwise.
        """
        if url not in self._cache:
            logger.debug(f"URL not in cache: {url}")
            return None

        filename = self._cache[url].filename

        # Try local dir first (during scraping)
        local_path = self.docs_dir / filename
        if local_path.exists():
            logger.debug(f"Read {filename} from local dir")
            return local_path.read_text(encoding="utf-8")

        # Fall back to ZIP (after finalize)
        if not self.zip_path.exists():
            logger.warning(f"Neither local file nor ZIP found for: {url}")
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
