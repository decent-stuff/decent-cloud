"""Base scraper class for provider scrapers."""

import json
import logging
from abc import ABC, abstractmethod
from pathlib import Path

import httpx
from crawl4ai import AsyncWebCrawler

from scraper.crawler import DEFAULT_BROWSER_CONFIG, create_crawl_config
from scraper.csv_writer import write_offerings_csv
from scraper.discovery import DiscoveryError, discover_sitemap, discover_via_crawl
from scraper.models import Offering
from scraper.storage import DocsArchive

logger = logging.getLogger(__name__)


class DocsScrapeError(Exception):
    """Raised when docs scraping fails."""


class BaseScraper(ABC):
    """Abstract base class for provider scrapers."""

    # Subclasses must define these
    provider_name: str
    provider_website: str
    docs_base_url: str | None = None  # Optional: base URL for docs discovery

    # Optional: URLs for Q&A generation (alternative to docs crawling)
    faq_urls: list[str] = []  # Specific pages to fetch for Q&A context
    changelog_url: str | None = None  # URL to check for updates (uses ETag)
    github_docs_repo: str | None = None  # GitHub repo with markdown docs (e.g., "owner/repo")
    github_docs_path: str = "tutorials"  # Path within repo to fetch docs from

    def __init__(self, output_dir: Path | None = None) -> None:
        """Initialize the scraper with an output directory."""
        self.output_dir = output_dir or Path("output") / self.provider_id
        self.archive = DocsArchive(self.output_dir)

    @property
    def provider_id(self) -> str:
        """Generate a safe provider ID from the name."""
        return self.provider_name.lower().replace(" ", "-")

    @abstractmethod
    async def scrape_offerings(self) -> list[Offering]:
        """Scrape offerings from the provider. Must be implemented by subclasses.

        Raises:
            Exception: Subclasses should raise specific errors on failure.
        """
        ...

    async def discover_doc_urls(self) -> list[str]:
        """Discover documentation URLs via sitemap or deep crawl.

        Uses docs_base_url if set, otherwise uses provider_website.
        Tries sitemap first, falls back to deep crawl if no sitemap found.

        Returns:
            List of discovered doc URLs.

        Raises:
            DiscoveryError: If discovery fails (rate limited, server error, etc.)
        """
        base_url = self.docs_base_url or self.provider_website
        logger.info(f"Discovering doc URLs for {self.provider_name} from {base_url}")

        # Try sitemap first (may raise DiscoveryError)
        urls = await discover_sitemap(base_url)
        if urls:
            logger.info(f"Found {len(urls)} URLs via sitemap")
            return self._filter_doc_urls(urls)

        # Fall back to deep crawl (may raise DiscoveryError)
        logger.info(f"No sitemap found, using deep crawl (max_depth=2, max_pages=50)")
        urls = await discover_via_crawl(base_url, max_depth=2, max_pages=50)
        logger.info(f"Found {len(urls)} URLs via deep crawl")
        return self._filter_doc_urls(urls)

    def _filter_doc_urls(self, urls: list[str]) -> list[str]:
        """Filter URLs to only include pages from the docs base URL domain.

        Default implementation keeps URLs that start with docs_base_url.
        Subclasses can override for provider-specific filtering.

        Args:
            urls: List of URLs to filter.

        Returns:
            Filtered list of doc URLs (deduplicated).
        """
        base_url = self.docs_base_url or self.provider_website
        # Normalize: ensure base_url ends without slash for consistent prefix matching
        base_url = base_url.rstrip("/")

        # Keep only URLs from the docs domain, deduplicate
        seen = set()
        filtered = []
        for url in urls:
            normalized = url.rstrip("/")
            if normalized.startswith(base_url) and normalized not in seen:
                seen.add(normalized)
                filtered.append(url)

        logger.debug(f"Filtered {len(urls)} URLs down to {len(filtered)} doc URLs from {base_url}")
        return filtered

    async def scrape_docs(self) -> int:
        """Scrape documentation pages and save to archive.

        Returns:
            Number of new/changed documents written.

        Raises:
            DiscoveryError: If URL discovery fails.
            DocsScrapeError: If no docs could be scraped successfully.
        """
        urls = await self.discover_doc_urls()
        if not urls:
            raise DocsScrapeError(f"No doc URLs found for {self.provider_name}")

        logger.info(f"Scraping {len(urls)} doc pages for {self.provider_name}")
        config = create_crawl_config()
        written_count = 0
        error_count = 0

        async with AsyncWebCrawler(config=DEFAULT_BROWSER_CONFIG) as crawler:
            for i, url in enumerate(urls, 1):
                try:
                    logger.debug(f"Crawling [{i}/{len(urls)}]: {url}")
                    result = await crawler.arun(url=url, config=config)

                    # Check for rate limiting in error message
                    if result.error_message:
                        if "429" in result.error_message or "rate" in result.error_message.lower():
                            raise DocsScrapeError(f"Rate limited while scraping docs: {url}")

                    if not result.success:
                        logger.warning(f"Failed to crawl {url}: {result.error_message}")
                        error_count += 1
                        continue

                    # Extract content - prefer fit_markdown, fall back to raw
                    fit_md = result.markdown.fit_markdown or ""
                    raw_md = result.markdown.raw_markdown or ""

                    # Use fit if it has substantial content, else fall back to raw
                    # (fit can be empty if pruning was too aggressive)
                    if len(fit_md.strip()) >= 100:
                        content = fit_md
                    elif raw_md.strip():
                        content = raw_md
                        logger.debug(f"Using raw markdown for {url} (fit was too short: {len(fit_md)} chars)")
                    else:
                        logger.warning(f"No markdown content for {url} (fit={len(fit_md)}, raw={len(raw_md)})")
                        error_count += 1
                        continue

                    # Extract topic from URL or page title
                    topic = self._extract_topic(url, result.metadata.get("title", ""))

                    # Get ETag from response headers if available
                    etag = result.metadata.get("etag")

                    # Check if changed before writing
                    if not self.archive.has_changed(url, etag, content):
                        logger.debug(f"Skipping unchanged: {url}")
                        continue

                    # Write to archive
                    self.archive.write(url, content, topic, etag)
                    written_count += 1

                except DocsScrapeError:
                    raise
                except Exception as e:
                    logger.error(f"Error crawling {url}: {e}")
                    error_count += 1

        logger.info(f"Wrote {written_count} new/changed docs for {self.provider_name}")

        # If all URLs failed, raise an error
        if error_count == len(urls) and written_count == 0:
            raise DocsScrapeError(f"All {len(urls)} doc pages failed to scrape for {self.provider_name}")

        return written_count

    def _extract_topic(self, url: str, title: str) -> str:
        """Extract topic from URL or page title.

        Args:
            url: The page URL.
            title: The page title.

        Returns:
            Topic string for filename generation.
        """
        # Use title if available and reasonable length
        if title and len(title) < 100:
            return title

        # Extract last path segment from URL
        parts = url.rstrip("/").split("/")
        if len(parts) > 3:
            return parts[-1]

        return "index"

    async def run(
        self,
        skip_offerings: bool = False,
        skip_docs: bool = False,
        keep_local: bool = False,
    ) -> tuple[Path | None, int]:
        """Run the full scraping process and write output files.

        Args:
            skip_offerings: If True, skip offerings scraping on failure.
            skip_docs: If True, skip docs scraping on failure.
            keep_local: If True, keep local docs/ directory for troubleshooting.

        Returns:
            Tuple of (csv_path, docs_count). csv_path is None if offerings skipped.

        Raises:
            Exception: If scraping fails and skip_* is False.
        """
        logger.info(f"Starting scrape for {self.provider_name}")

        csv_path = None
        docs_count = 0

        # Scrape offerings
        try:
            offerings = await self.scrape_offerings()
            csv_path = self.output_dir / "offerings.csv"
            write_offerings_csv(offerings, csv_path)
            logger.info(f"Wrote {len(offerings)} offerings to {csv_path}")
        except Exception as e:
            if skip_offerings:
                logger.warning(f"Offerings scrape failed (skipped): {e}")
            else:
                raise

        # Scrape docs
        try:
            docs_count = await self.scrape_docs()
        except Exception as e:
            if skip_docs:
                logger.warning(f"Docs scrape failed (skipped): {e}")
            else:
                raise

        # Finalize: create ZIP from local docs
        self.archive.finalize(keep_local=keep_local)

        return csv_path, docs_count

    # --- Q&A Generation Methods ---

    def _metadata_path(self) -> Path:
        """Path to metadata JSON file for change detection."""
        return self.output_dir / "metadata.json"

    def _load_metadata(self) -> dict[str, str]:
        """Load stored metadata (ETags, timestamps)."""
        path = self._metadata_path()
        if path.exists():
            try:
                return json.loads(path.read_text())
            except (json.JSONDecodeError, OSError):
                pass
        return {}

    def _save_metadata(self, metadata: dict[str, str]) -> None:
        """Save metadata for change detection."""
        self.output_dir.mkdir(parents=True, exist_ok=True)
        self._metadata_path().write_text(json.dumps(metadata, indent=2))

    async def check_for_changes(self) -> bool:
        """Check if content has changed since last scrape.

        Uses ETag from changelog_url if available.

        Returns:
            True if changes detected or no previous metadata exists.
        """
        if not self.changelog_url:
            logger.debug(f"No changelog_url for {self.provider_name}, assuming changed")
            return True

        metadata = self._load_metadata()
        stored_etag = metadata.get("changelog_etag")

        try:
            async with httpx.AsyncClient(timeout=30.0) as client:
                response = await client.head(self.changelog_url)
                current_etag = response.headers.get("etag")

                if not current_etag:
                    logger.debug(f"No ETag from {self.changelog_url}, assuming changed")
                    return True

                if current_etag != stored_etag:
                    logger.info(f"ETag changed for {self.provider_name}: {stored_etag} -> {current_etag}")
                    return True

                logger.info(f"No changes detected for {self.provider_name} (ETag: {current_etag})")
                return False

        except httpx.HTTPError as e:
            logger.warning(f"Failed to check changelog: {e}, assuming changed")
            return True

    async def fetch_github_docs(self, max_files: int = 30) -> str:
        """Fetch markdown docs from GitHub repository.

        Args:
            max_files: Maximum number of markdown files to fetch.

        Returns:
            Combined markdown content from GitHub docs.
        """
        if not self.github_docs_repo:
            return ""

        contents: list[str] = []
        api_url = f"https://api.github.com/repos/{self.github_docs_repo}/contents/{self.github_docs_path}"

        async with httpx.AsyncClient(timeout=30.0) as client:
            # List directory contents
            try:
                response = await client.get(api_url)
                if response.status_code != 200:
                    logger.warning(f"Failed to list GitHub docs: {response.status_code}")
                    return ""

                items = response.json()
                if not isinstance(items, list):
                    logger.warning(f"Unexpected GitHub API response: {type(items)}")
                    return ""

            except httpx.HTTPError as e:
                logger.warning(f"GitHub API error: {e}")
                return ""

            # Fetch markdown files (look for .md files or directories with 01.en.md)
            fetched = 0
            for item in items:
                if fetched >= max_files:
                    break

                if item.get("type") == "dir":
                    # Check for tutorial format: {name}/01.en.md
                    md_url = f"https://raw.githubusercontent.com/{self.github_docs_repo}/master/{self.github_docs_path}/{item['name']}/01.en.md"
                    try:
                        md_response = await client.get(md_url)
                        if md_response.status_code == 200:
                            content = md_response.text
                            if content.strip():
                                contents.append(f"## Tutorial: {item['name']}\n\n{content[:4000]}")
                                fetched += 1
                                logger.debug(f"Fetched GitHub doc: {item['name']}")
                    except httpx.HTTPError:
                        pass

                elif item.get("name", "").endswith(".md"):
                    # Direct markdown file
                    try:
                        md_response = await client.get(item["download_url"])
                        if md_response.status_code == 200:
                            content = md_response.text
                            if content.strip():
                                contents.append(f"## Doc: {item['name']}\n\n{content[:4000]}")
                                fetched += 1
                    except httpx.HTTPError:
                        pass

            logger.info(f"Fetched {fetched} docs from GitHub repo {self.github_docs_repo}")

        return "\n\n---\n\n".join(contents)

    async def fetch_faq_content(self) -> str:
        """Fetch content from faq_urls and github_docs_repo for Q&A generation.

        Returns:
            Combined markdown content from all sources.
        """
        contents: list[str] = []

        # Fetch from GitHub docs (most reliable, no bot protection)
        github_content = await self.fetch_github_docs()
        if github_content:
            contents.append(github_content)

        # Fetch from FAQ URLs using Crawl4AI
        if self.faq_urls:
            config = create_crawl_config()
            async with AsyncWebCrawler(config=DEFAULT_BROWSER_CONFIG) as crawler:
                for url in self.faq_urls:
                    try:
                        logger.debug(f"Fetching FAQ content from {url}")
                        result = await crawler.arun(url=url, config=config)

                        if not result.success:
                            logger.warning(f"Failed to fetch {url}: {result.error_message}")
                            continue

                        # Prefer fit_markdown, fall back to raw
                        content = result.markdown.fit_markdown or result.markdown.raw_markdown or ""
                        if content.strip():
                            contents.append(f"## Source: {url}\n\n{content}")

                    except Exception as e:
                        logger.warning(f"Error fetching {url}: {e}")

        return "\n\n---\n\n".join(contents)

    async def generate_qa(
        self,
        offerings: list[Offering],
        force: bool = False,
    ) -> Path | None:
        """Generate Q&A content using LLM.

        Args:
            offerings: List of offerings for context.
            force: If True, regenerate even if no changes detected.

        Returns:
            Path to generated qa.json file, or None if skipped.
        """
        from scraper.qa_generator import QAGenerator, QAGeneratorError

        # Check for changes unless forced
        if not force:
            has_changes = await self.check_for_changes()
            if not has_changes:
                logger.info(f"Skipping Q&A generation for {self.provider_name} (no changes)")
                return None

        # Fetch FAQ content
        docs_content = await self.fetch_faq_content()

        # Generate Q&A
        try:
            generator = QAGenerator()
            qa_pairs = generator.generate_qa(
                provider_name=self.provider_name,
                provider_website=self.provider_website,
                offerings=offerings,
                docs_content=docs_content,
            )
        except QAGeneratorError as e:
            logger.error(f"Q&A generation failed: {e}")
            raise

        # Save Q&A
        qa_path = self.output_dir / "qa.json"
        self.output_dir.mkdir(parents=True, exist_ok=True)
        qa_path.write_text(json.dumps(qa_pairs, indent=2))
        logger.info(f"Wrote {len(qa_pairs)} Q&A pairs to {qa_path}")

        # Update metadata with current ETag
        if self.changelog_url:
            try:
                async with httpx.AsyncClient(timeout=30.0) as client:
                    response = await client.head(self.changelog_url)
                    etag = response.headers.get("etag")
                    if etag:
                        metadata = self._load_metadata()
                        metadata["changelog_etag"] = etag
                        self._save_metadata(metadata)
            except httpx.HTTPError:
                pass

        return qa_path
