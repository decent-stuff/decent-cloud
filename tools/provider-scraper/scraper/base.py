"""Base scraper class for provider scrapers."""

import logging
from abc import ABC, abstractmethod
from pathlib import Path

from crawl4ai import AsyncWebCrawler

from scraper.crawler import DEFAULT_BROWSER_CONFIG, create_crawl_config
from scraper.csv_writer import write_offerings_csv
from scraper.discovery import discover_sitemap, discover_via_crawl
from scraper.models import Offering
from scraper.storage import DocsArchive

logger = logging.getLogger(__name__)


class BaseScraper(ABC):
    """Abstract base class for provider scrapers."""

    # Subclasses must define these
    provider_name: str
    provider_website: str
    docs_base_url: str | None = None  # Optional: base URL for docs discovery

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
        """Scrape offerings from the provider. Must be implemented by subclasses."""
        ...

    async def discover_doc_urls(self) -> list[str]:
        """Discover documentation URLs via sitemap or deep crawl.

        Uses docs_base_url if set, otherwise uses provider_website.
        Tries sitemap first, falls back to deep crawl if no sitemap found.

        Returns:
            List of discovered doc URLs.
        """
        base_url = self.docs_base_url or self.provider_website
        logger.info(f"Discovering doc URLs for {self.provider_name} from {base_url}")

        # Try sitemap first
        urls = await discover_sitemap(base_url)
        if urls:
            logger.info(f"Found {len(urls)} URLs via sitemap")
            return self._filter_doc_urls(urls)

        # Fall back to deep crawl
        logger.info(f"No sitemap found, using deep crawl (max_depth=2, max_pages=50)")
        urls = await discover_via_crawl(base_url, max_depth=2, max_pages=50)
        logger.info(f"Found {len(urls)} URLs via deep crawl")
        return self._filter_doc_urls(urls)

    def _filter_doc_urls(self, urls: list[str]) -> list[str]:
        """Filter URLs to only include docs/help pages.

        Default implementation keeps URLs containing common doc path segments.
        Subclasses can override for provider-specific filtering.

        Args:
            urls: List of URLs to filter.

        Returns:
            Filtered list of doc URLs.
        """
        doc_patterns = ["/docs", "/help", "/support", "/guide", "/faq", "/tutorial", "/knowledge"]
        filtered = [url for url in urls if any(pattern in url.lower() for pattern in doc_patterns)]
        logger.debug(f"Filtered {len(urls)} URLs down to {len(filtered)} doc URLs")
        return filtered

    async def scrape_docs(self) -> int:
        """Scrape documentation pages and save to archive.

        Returns:
            Number of new/changed documents written.
        """
        urls = await self.discover_doc_urls()
        if not urls:
            logger.warning(f"No doc URLs found for {self.provider_name}")
            return 0

        logger.info(f"Scraping {len(urls)} doc pages for {self.provider_name}")
        config = create_crawl_config()
        written_count = 0

        async with AsyncWebCrawler(config=DEFAULT_BROWSER_CONFIG) as crawler:
            for i, url in enumerate(urls, 1):
                try:
                    logger.debug(f"Crawling [{i}/{len(urls)}]: {url}")
                    result = await crawler.arun(url=url, config=config)

                    if not result.success:
                        logger.warning(f"Failed to crawl {url}: {result.error_message}")
                        continue

                    # Extract content - prefer fit_markdown over raw
                    content = result.markdown.fit_markdown or result.markdown.raw_markdown
                    if not content:
                        logger.warning(f"No markdown content for {url}")
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

                except Exception as e:
                    logger.error(f"Error crawling {url}: {e}")

        logger.info(f"Wrote {written_count} new/changed docs for {self.provider_name}")
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

    async def run(self) -> tuple[Path, int]:
        """Run the full scraping process and write output files.

        Returns:
            Tuple of (csv_path, docs_count)
        """
        logger.info(f"Starting scrape for {self.provider_name}")

        # Scrape offerings
        offerings = await self.scrape_offerings()
        csv_path = self.output_dir / "offerings.csv"
        write_offerings_csv(offerings, csv_path)
        logger.info(f"Wrote {len(offerings)} offerings to {csv_path}")

        # Scrape docs
        docs_count = await self.scrape_docs()

        return csv_path, docs_count
