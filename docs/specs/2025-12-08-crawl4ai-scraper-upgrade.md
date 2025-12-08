# Provider Scraper Upgrade: Crawl4AI Integration

**Status**: Draft
**Created**: 2025-12-08
**Author**: Claude Code

## Summary

Replace the rudimentary httpx/BeautifulSoup-based scraper with Crawl4AI for professional-grade web crawling optimized for LLM consumption. This enables sitemap-based discovery, incremental crawling, JS-rendered content, and high-quality markdown generation.

## Current State

The existing `tools/provider-scraper` has significant limitations:

| Component | Current | Issues |
|-----------|---------|--------|
| HTTP Client | `httpx.Client` (sync) | No JS rendering, no session persistence |
| HTML→Markdown | Custom `markdown.py` (~340 lines) | Basic, misses edge cases, manual noise removal |
| Discovery | Hardcoded URL lists | No sitemap support, no link following |
| Caching | None | Re-fetches everything on every run |
| Crawling | Single-page fetch | No depth crawling, no incremental updates |

## Proposed Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Provider Scraper v2                          │
├─────────────────────────────────────────────────────────────────┤
│  scraper/                                                        │
│  ├── crawler.py          # Crawl4AI wrapper with our defaults   │
│  ├── discovery.py        # Sitemap + deep crawl URL discovery   │
│  ├── cache.py            # ETag/Last-Modified tracking          │
│  ├── models.py           # (unchanged) Offering model           │
│  ├── csv_writer.py       # (unchanged) CSV export               │
│  ├── base.py             # (simplified) Base provider class     │
│  ├── cli.py              # (improved) Async CLI with progress   │
│  └── providers/                                                  │
│      ├── hetzner.py      # Uses new crawler                     │
│      ├── contabo.py                                              │
│      └── ovh.py                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Core Crawler Module (`scraper/crawler.py`)

Create a thin wrapper around Crawl4AI with project-specific defaults:

```python
"""Crawl4AI wrapper with LLM-optimized defaults."""

from crawl4ai import AsyncWebCrawler, BrowserConfig, CrawlerRunConfig, CacheMode
from crawl4ai.content_filter_strategy import PruningContentFilter
from crawl4ai.markdown_generation_strategy import DefaultMarkdownGenerator
from crawl4ai.deep_crawling import BFSDeepCrawlStrategy

# Project defaults
DEFAULT_BROWSER_CONFIG = BrowserConfig(
    headless=True,
    browser_type="chromium",
    verbose=False,
)

def create_markdown_generator(threshold: float = 0.45) -> DefaultMarkdownGenerator:
    """Create markdown generator with noise filtering."""
    return DefaultMarkdownGenerator(
        content_filter=PruningContentFilter(
            threshold=threshold,
            threshold_type="dynamic",
            min_word_threshold=10,
        )
    )

def create_crawl_config(
    cache_mode: CacheMode = CacheMode.ENABLED,
    max_depth: int = 0,
    max_pages: int = 100,
) -> CrawlerRunConfig:
    """Create crawl config with sensible defaults."""
    config = CrawlerRunConfig(
        cache_mode=cache_mode,
        markdown_generator=create_markdown_generator(),
        word_count_threshold=10,
        excluded_tags=["nav", "footer", "aside", "header"],
        remove_overlay_elements=True,
        exclude_external_links=True,
    )

    if max_depth > 0:
        config.deep_crawl_strategy = BFSDeepCrawlStrategy(
            max_depth=max_depth,
            include_external=False,
            max_pages=max_pages,
        )

    return config
```

### Phase 2: URL Discovery (`scraper/discovery.py`)

Sitemap-first discovery with fallback to deep crawl:

```python
"""URL discovery via sitemap or deep crawl."""

import asyncio
from xml.etree import ElementTree
from crawl4ai import AsyncWebCrawler
from crawl4ai.deep_crawling import BFSDeepCrawlStrategy

SITEMAP_PATHS = [
    "/sitemap.xml",
    "/sitemap_index.xml",
    "/sitemap/sitemap.xml",
    "/docs/sitemap.xml",
]

async def discover_sitemap(base_url: str) -> list[str] | None:
    """Try to find and parse sitemap, return URLs or None."""
    async with AsyncWebCrawler() as crawler:
        for path in SITEMAP_PATHS:
            url = f"{base_url.rstrip('/')}{path}"
            result = await crawler.arun(url)
            if result.success and result.html:
                urls = parse_sitemap_xml(result.html)
                if urls:
                    return urls
    return None

def parse_sitemap_xml(xml_content: str) -> list[str]:
    """Parse sitemap XML, handling both index and urlset."""
    urls = []
    try:
        root = ElementTree.fromstring(xml_content)
        ns = {"sm": "http://www.sitemaps.org/schemas/sitemap/0.9"}

        # Check for sitemap index
        for sitemap in root.findall(".//sm:sitemap/sm:loc", ns):
            urls.append(sitemap.text)  # Recurse on these

        # Check for URL entries
        for loc in root.findall(".//sm:url/sm:loc", ns):
            urls.append(loc.text)
    except ElementTree.ParseError:
        pass
    return urls

async def discover_via_crawl(
    base_url: str,
    max_depth: int = 2,
    max_pages: int = 100,
) -> list[str]:
    """Deep crawl to discover URLs when no sitemap exists."""
    config = CrawlerRunConfig(
        deep_crawl_strategy=BFSDeepCrawlStrategy(
            max_depth=max_depth,
            include_external=False,
            max_pages=max_pages,
        )
    )

    async with AsyncWebCrawler() as crawler:
        results = await crawler.arun(base_url, config=config)
        return [r.url for r in results if r.success]
```

### Phase 3: Incremental Cache (`scraper/cache.py`)

Track content hashes for change detection:

```python
"""Content-based caching for incremental crawls."""

import hashlib
import json
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

@dataclass
class CacheEntry:
    url: str
    content_hash: str
    etag: str | None
    last_modified: str | None
    crawled_at: datetime

class CrawlCache:
    """Track crawled URLs and detect changes."""

    def __init__(self, cache_file: Path):
        self.cache_file = cache_file
        self.entries: dict[str, CacheEntry] = {}
        self._load()

    def _load(self) -> None:
        if self.cache_file.exists():
            data = json.loads(self.cache_file.read_text())
            for url, entry in data.items():
                self.entries[url] = CacheEntry(**entry)

    def save(self) -> None:
        data = {url: vars(e) for url, e in self.entries.items()}
        self.cache_file.parent.mkdir(parents=True, exist_ok=True)
        self.cache_file.write_text(json.dumps(data, default=str))

    def content_hash(self, content: str) -> str:
        return hashlib.sha256(content.encode()).hexdigest()[:16]

    def has_changed(self, url: str, content: str) -> bool:
        """Return True if content is new or changed."""
        new_hash = self.content_hash(content)
        if url not in self.entries:
            return True
        return self.entries[url].content_hash != new_hash

    def update(self, url: str, content: str, etag: str | None = None) -> None:
        self.entries[url] = CacheEntry(
            url=url,
            content_hash=self.content_hash(content),
            etag=etag,
            last_modified=None,
            crawled_at=datetime.utcnow(),
        )
```

### Phase 4: Simplified Base Class (`scraper/base.py`)

Refactor to async with Crawl4AI:

```python
"""Base scraper class using Crawl4AI."""

from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from crawl4ai import AsyncWebCrawler, CrawlerRunConfig, CacheMode

from .cache import CrawlCache
from .crawler import create_crawl_config, DEFAULT_BROWSER_CONFIG
from .csv_writer import write_offerings_csv
from .discovery import discover_sitemap, discover_via_crawl
from .models import Offering

@dataclass
class MarkdownDoc:
    """Crawled markdown document."""
    content: str
    url: str
    provider: str
    topic: str
    crawled_at: datetime

class BaseScraper(ABC):
    """Abstract base for provider scrapers."""

    provider_name: str
    provider_website: str
    docs_base_url: str | None = None  # Override for docs discovery

    def __init__(self, output_dir: Path | None = None):
        self.output_dir = output_dir or Path("output") / self.provider_id
        self.cache = CrawlCache(self.output_dir / ".crawl_cache.json")

    @property
    def provider_id(self) -> str:
        return self.provider_name.lower().replace(" ", "-")

    @abstractmethod
    async def scrape_offerings(self) -> list[Offering]:
        """Scrape product offerings. Must be implemented."""
        ...

    async def discover_doc_urls(self) -> list[str]:
        """Discover documentation URLs via sitemap or crawl."""
        base = self.docs_base_url or self.provider_website

        # Try sitemap first
        urls = await discover_sitemap(base)
        if urls:
            return self._filter_doc_urls(urls)

        # Fall back to deep crawl
        return await discover_via_crawl(base, max_depth=2, max_pages=50)

    def _filter_doc_urls(self, urls: list[str]) -> list[str]:
        """Filter URLs to only include docs/help pages."""
        keywords = ["docs", "help", "faq", "support", "guide", "tutorial"]
        return [u for u in urls if any(k in u.lower() for k in keywords)]

    async def scrape_docs(self) -> list[MarkdownDoc]:
        """Crawl documentation and convert to markdown."""
        urls = await self.discover_doc_urls()
        docs = []

        config = create_crawl_config(cache_mode=CacheMode.ENABLED)

        async with AsyncWebCrawler(config=DEFAULT_BROWSER_CONFIG) as crawler:
            for url in urls:
                result = await crawler.arun(url, config=config)
                if not result.success:
                    continue

                # Use fit_markdown (filtered) if available
                content = result.markdown.fit_markdown or result.markdown.raw_markdown

                # Skip if unchanged
                if not self.cache.has_changed(url, content):
                    continue

                self.cache.update(url, content)

                docs.append(MarkdownDoc(
                    content=content,
                    url=url,
                    provider=self.provider_name,
                    topic=self._extract_topic(url, result.metadata),
                    crawled_at=datetime.utcnow(),
                ))

        self.cache.save()
        return docs

    def _extract_topic(self, url: str, metadata: dict) -> str:
        """Extract topic from URL path or page title."""
        if title := metadata.get("title"):
            return title
        # Fall back to URL path
        path = url.rstrip("/").split("/")[-1]
        return path.replace("-", " ").replace("_", " ").title()

    async def run(self) -> tuple[Path, list[Path]]:
        """Run full scrape and write outputs."""
        # Scrape offerings
        offerings = await self.scrape_offerings()
        csv_path = self.output_dir / "offerings.csv"
        write_offerings_csv(offerings, csv_path)

        # Scrape docs
        docs = await self.scrape_docs()
        md_paths = self._write_docs(docs)

        return csv_path, md_paths

    def _write_docs(self, docs: list[MarkdownDoc]) -> list[Path]:
        """Write markdown docs with frontmatter."""
        docs_dir = self.output_dir / "docs"
        docs_dir.mkdir(parents=True, exist_ok=True)
        paths = []

        for doc in docs:
            # Safe filename from topic
            safe_name = "".join(c if c.isalnum() or c in "-_ " else "" for c in doc.topic)
            safe_name = safe_name.lower().replace(" ", "-")[:50]
            path = docs_dir / f"{safe_name}.md"

            frontmatter = f"""---
source: {doc.url}
provider: {doc.provider}
topic: {doc.topic}
crawled: {doc.crawled_at.strftime('%Y-%m-%d')}
---

"""
            path.write_text(frontmatter + doc.content)
            paths.append(path)

        return paths
```

### Phase 5: Updated CLI (`scraper/cli.py`)

Async CLI with progress reporting:

```python
"""Async CLI for provider scrapers."""

import asyncio
import sys
from pathlib import Path

from .providers.hetzner import HetznerScraper
from .providers.contabo import ContaboScraper
from .providers.ovh import OvhScraper

SCRAPERS = {
    "hetzner": HetznerScraper,
    "contabo": ContaboScraper,
    "ovh": OvhScraper,
}

async def run_scraper(name: str, output_base: Path) -> tuple[int, int]:
    """Run a single scraper, return (offerings_count, docs_count)."""
    scraper_cls = SCRAPERS[name]
    scraper = scraper_cls(output_base / name)

    print(f"\n=== Scraping {name} ===")
    csv_path, md_paths = await scraper.run()

    # Count offerings
    with csv_path.open() as f:
        offerings = sum(1 for _ in f) - 1

    print(f"  Offerings: {offerings}")
    print(f"  Docs: {len(md_paths)} files")

    return offerings, len(md_paths)

async def main() -> None:
    output_base = Path(__file__).parent.parent / "output"
    providers = sys.argv[1:] if len(sys.argv) > 1 else list(SCRAPERS.keys())

    # Validate
    for p in providers:
        if p not in SCRAPERS:
            print(f"Unknown provider: {p}")
            print(f"Available: {', '.join(SCRAPERS.keys())}")
            sys.exit(1)

    # Run all (could parallelize with asyncio.gather)
    total_offerings = 0
    total_docs = 0

    for provider in providers:
        offerings, docs = await run_scraper(provider, output_base)
        total_offerings += offerings
        total_docs += docs

    print(f"\n=== Summary ===")
    print(f"Total: {total_offerings} offerings, {total_docs} docs")

def cli() -> None:
    asyncio.run(main())

if __name__ == "__main__":
    cli()
```

### Phase 6: Update Dependencies (`pyproject.toml`)

```toml
[project]
name = "provider-scraper"
version = "0.2.0"
description = "Scrape hosting provider offerings and docs for catalog seeding"
requires-python = ">=3.11"
dependencies = [
    "crawl4ai>=0.4.0",
    "pydantic>=2.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=8.0",
    "pytest-asyncio>=0.24",
    "ruff>=0.8",
    "pyright>=1.1",
]

[project.scripts]
scrape = "scraper.cli:cli"
```

## Migration Steps

1. **Install Crawl4AI**: `pip install crawl4ai` (requires Playwright browsers)
2. **Run browser install**: `playwright install chromium`
3. **Implement modules** in order: crawler → discovery → cache → base → cli
4. **Update each provider** to use new async base class
5. **Test incrementally** with one provider first (e.g., Hetzner)
6. **Remove old code**: Delete `markdown.py` (replaced by Crawl4AI)

## Key Improvements

| Feature | Before | After |
|---------|--------|-------|
| Markdown quality | Basic regex/BS4 | Crawl4AI fit_markdown with PruningContentFilter |
| JS rendering | None | Chromium via Playwright |
| Discovery | Hardcoded URLs | Sitemap parsing + BFS deep crawl |
| Caching | None | Content-hash based incremental |
| Concurrency | Sync, sequential | Async, parallelizable |
| Noise removal | Manual skip patterns | Dynamic threshold filtering |

## Estimated Effort

| Phase | Effort |
|-------|--------|
| 1. Core Crawler | 1 hour |
| 2. Discovery | 1 hour |
| 3. Cache | 30 min |
| 4. Base Class | 1 hour |
| 5. CLI | 30 min |
| 6. Provider Updates | 1 hour |
| Testing | 1 hour |
| **Total** | ~6 hours |

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Crawl4AI API changes | Pin version in requirements |
| Rate limiting by providers | Add configurable delays, respect robots.txt |
| Playwright browser size | Use Docker for CI, lightweight for local |
| Some sites block headless | Use stealth mode, rotate user agents |

## Success Criteria

- [ ] All existing providers work with new crawler
- [ ] Markdown output quality is equal or better
- [ ] Incremental crawling skips unchanged pages
- [ ] Sitemap discovery works for providers with sitemaps
- [ ] Deep crawl discovers docs for providers without sitemaps
- [ ] Tests pass, `cargo make` clean (for overall project)
