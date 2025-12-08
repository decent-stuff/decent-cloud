"""Provider scraper framework for catalog seeding."""

from scraper.base import BaseScraper
from scraper.crawler import (
    DEFAULT_BROWSER_CONFIG,
    DEFAULT_PRUNING_THRESHOLD,
    DEFAULT_WORD_THRESHOLD,
    create_crawl_config,
    create_markdown_generator,
)
from scraper.csv_writer import write_offerings_csv
from scraper.discovery import discover_sitemap, discover_via_crawl
from scraper.models import Offering
from scraper.storage import CacheEntry, DocsArchive

__all__ = [
    "BaseScraper",
    "CacheEntry",
    "DEFAULT_BROWSER_CONFIG",
    "DEFAULT_PRUNING_THRESHOLD",
    "DEFAULT_WORD_THRESHOLD",
    "DocsArchive",
    "Offering",
    "create_crawl_config",
    "create_markdown_generator",
    "discover_sitemap",
    "discover_via_crawl",
    "write_offerings_csv",
]
