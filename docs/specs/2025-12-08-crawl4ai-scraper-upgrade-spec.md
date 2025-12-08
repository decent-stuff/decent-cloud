# Provider Scraper Upgrade: Crawl4AI Integration

**Status:** In Progress
**Created:** 2025-12-08
**Author:** Claude Code (Orchestrator)

## Summary

Replace httpx/BeautifulSoup scraper with Crawl4AI for professional-grade web crawling. Implement ZIP-based document storage with ETag/content-hash caching for efficient incremental crawls.

## Requirements

### Must-have
- [ ] Core crawler module wrapping Crawl4AI with project defaults
- [ ] URL discovery via sitemap parsing + BFS deep crawl fallback
- [ ] ZIP archive storage (one file per page, single archive per provider)
- [ ] ETag + content-hash based incremental caching
- [ ] Async base class with Crawl4AI integration
- [ ] Updated CLI with async support
- [ ] All existing providers migrated to new architecture
- [ ] Tests for all new modules

### Nice-to-have
- [ ] Parallel provider crawling
- [ ] Progress reporting during crawl
- [ ] Configurable rate limiting

## Architecture

```
output/
├── hetzner/
│   ├── offerings.csv         # Product catalog (unchanged)
│   ├── docs.zip              # All markdown docs in single archive
│   │   ├── cloud-servers.md
│   │   ├── pricing.md
│   │   └── ...
│   └── cache.json            # URL → {etag, content_hash, filename, crawled_at}
```

**Cache schema:**
```json
{
  "https://docs.hetzner.com/cloud/servers": {
    "filename": "cloud-servers.md",
    "etag": "\"5d8c72a5edda8\"",
    "content_hash": "a1b2c3...",
    "crawled_at": "2025-12-08T12:00:00Z"
  }
}
```

## Steps

### Step 1: Core Crawler Module (`scraper/crawler.py`)
**Success:** Module exists with `create_crawl_config()`, `create_markdown_generator()`, and `DEFAULT_BROWSER_CONFIG`. Unit tests pass.
**Status:** Complete

### Step 2: URL Discovery Module (`scraper/discovery.py`)
**Success:** `discover_sitemap()` and `discover_via_crawl()` functions work. Sitemap XML parsing handles both index and urlset formats. Unit tests pass.
**Status:** Pending

### Step 3: ZIP Storage + Cache Module (`scraper/storage.py`)
**Success:** `DocsArchive` class can read/write markdown to ZIP, track ETags/hashes in cache.json, detect changes via `has_changed()`. Unit tests pass.
**Status:** Pending

### Step 4: Async Base Class (`scraper/base.py`)
**Success:** `BaseScraper` refactored to async, uses new crawler/discovery/storage modules. Abstract `scrape_offerings()` preserved. Unit tests pass.
**Status:** Pending

### Step 5: Migrate Providers
**Success:** All three providers (Hetzner, Contabo, OVH) use new async base class. Existing functionality preserved. Unit tests pass.
**Status:** Pending

### Step 6: Update CLI (`scraper/cli.py`)
**Success:** CLI runs async, supports single/all providers, shows progress. `cargo make` clean.
**Status:** Pending

### Step 7: Remove Old Code + Final Cleanup
**Success:** `markdown.py` deleted, unused imports removed, all tests pass, `cargo make` clean.
**Status:** Pending

## Execution Log

### Step 1
- **Implementation:** Created `/code/tools/provider-scraper/scraper/crawler.py` with:
  - `DEFAULT_BROWSER_CONFIG` - BrowserConfig(browser_type="chromium", headless=True, verbose=False)
  - `DEFAULT_PRUNING_THRESHOLD = 0.48`, `DEFAULT_WORD_THRESHOLD = 10` - Shared constants (DRY)
  - `create_markdown_generator(threshold, min_word_threshold)` - Returns DefaultMarkdownGenerator with PruningContentFilter
  - `create_crawl_config(cache_mode, excluded_tags, ...)` - Returns CrawlerRunConfig with sensible defaults
- **Tests:** Created `/code/tools/provider-scraper/tests/test_crawler.py` with 24 unit tests covering:
  - DEFAULT_BROWSER_CONFIG validation (4 tests)
  - create_markdown_generator() with default/custom thresholds (6 tests)
  - create_crawl_config() with all parameters (12 tests)
  - Integration tests combining components (2 tests)
- **Dependencies:** Updated `/code/tools/provider-scraper/pyproject.toml` to add `crawl4ai>=0.4.0`
- **Test Results:** All 24 tests passed successfully in 1.28s (2 warnings from crawl4ai's Pydantic v2 migration, not our code)
- **Review Findings:**
  - ✅ KISS/MINIMAL: Very clean, 72 lines total (was 65 before DRY refactor)
  - ✅ DRY: Extracted DEFAULT_PRUNING_THRESHOLD and DEFAULT_WORD_THRESHOLD to eliminate duplication
  - ✅ Tests comprehensive: Both positive/negative paths covered (no error conditions in factory functions)
  - ✅ Follows codebase patterns: Matches models.py style (type hints, Pydantic, docstrings)
  - ✅ Simplified: Added min_word_threshold parameter to create_markdown_generator() for consistency
- **Outcome:** SUCCESS - Core crawler module implemented with full test coverage and reviewed for quality

### Step 2
- **Implementation:** (pending)
- **Review:** (pending)
- **Outcome:** (pending)

### Step 3
- **Implementation:** (pending)
- **Review:** (pending)
- **Outcome:** (pending)

### Step 4
- **Implementation:** (pending)
- **Review:** (pending)
- **Outcome:** (pending)

### Step 5
- **Implementation:** (pending)
- **Review:** (pending)
- **Outcome:** (pending)

### Step 6
- **Implementation:** (pending)
- **Review:** (pending)
- **Outcome:** (pending)

### Step 7
- **Implementation:** (pending)
- **Review:** (pending)
- **Outcome:** (pending)

## Completion Summary
(To be filled in Phase 4)
