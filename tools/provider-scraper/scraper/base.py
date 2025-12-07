"""Base scraper class for provider scrapers."""

from abc import ABC, abstractmethod
from pathlib import Path

import httpx

from scraper.csv_writer import write_offerings_csv
from scraper.markdown import MarkdownDoc, chunk_markdown, write_markdown_docs
from scraper.models import Offering


class BaseScraper(ABC):
    """Abstract base class for provider scrapers."""

    # Subclasses must define these
    provider_name: str
    provider_website: str

    def __init__(self, output_dir: Path | None = None) -> None:
        """Initialize the scraper with an output directory."""
        self.output_dir = output_dir or Path("output") / self.provider_id
        self.client = httpx.Client(
            timeout=30.0,
            follow_redirects=True,
            headers={
                "User-Agent": "Mozilla/5.0 (compatible; ProviderScraper/1.0)",
                "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                "Accept-Language": "en-US,en;q=0.5",
            },
        )

    @property
    def provider_id(self) -> str:
        """Generate a safe provider ID from the name."""
        return self.provider_name.lower().replace(" ", "-")

    def fetch(self, url: str) -> str:
        """Fetch a URL and return the HTML content."""
        response = self.client.get(url)
        response.raise_for_status()
        return response.text

    @abstractmethod
    def scrape_offerings(self) -> list[Offering]:
        """Scrape offerings from the provider. Must be implemented by subclasses."""
        ...

    @abstractmethod
    def scrape_docs(self) -> list[MarkdownDoc]:
        """Scrape documentation/help content. Must be implemented by subclasses."""
        ...

    def run(self) -> tuple[Path, list[Path]]:
        """Run the full scraping process and write output files.

        Returns:
            Tuple of (csv_path, list of markdown_paths)
        """
        # Scrape offerings
        offerings = self.scrape_offerings()
        csv_path = self.output_dir / "offerings.csv"
        write_offerings_csv(offerings, csv_path)

        # Scrape docs
        docs = self.scrape_docs()
        docs_dir = self.output_dir / "docs"
        md_paths: list[Path] = []
        for doc in docs:
            chunks = chunk_markdown(doc)
            paths = write_markdown_docs(chunks, docs_dir)
            md_paths.extend(paths)

        return csv_path, md_paths

    def close(self) -> None:
        """Close the HTTP client."""
        self.client.close()

    def __enter__(self) -> "BaseScraper":
        return self

    def __exit__(self, *args: object) -> None:
        self.close()
