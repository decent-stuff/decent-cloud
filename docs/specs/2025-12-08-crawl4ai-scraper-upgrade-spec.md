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
- **Implementation:** Created `/code/tools/provider-scraper/scraper/discovery.py` with:
  - `SITEMAP_PATHS` - List of common sitemap locations: `/sitemap.xml`, `/sitemap_index.xml`, `/sitemap1.xml`, `/robots.txt`
  - `parse_sitemap_xml(xml_content: str) -> list[str]` - Parses both sitemap index and urlset formats using wildcard namespaces (`{*}`) for flexibility
  - `discover_sitemap(base_url: str) -> list[str] | None` - Tries common paths, handles robots.txt parsing, recursively fetches child sitemaps from sitemap index
  - `_fetch_sitemap_content(client, sitemap_url, path)` - Helper to fetch sitemap content with robots.txt handling (DRY)
  - `_fetch_child_sitemaps(client, sitemap_urls)` - Helper to fetch and parse child sitemaps from sitemap index (DRY)
  - `_extract_sitemaps_from_robots(robots_content: str) -> list[str]` - Helper to extract sitemap URLs from robots.txt
  - `discover_via_crawl(base_url: str, max_depth: int = 2, max_pages: int = 50) -> list[str]` - BFS deep crawl fallback using Crawl4AI
- **Tests:** Created `/code/tools/provider-scraper/tests/test_discovery.py` with 20 unit tests covering:
  - `parse_sitemap_xml()`: 14 tests (urlset/index formats with/without namespace, invalid XML, empty inputs, whitespace trimming, complex tags)
  - `_extract_sitemaps_from_robots()`: 6 tests (single/multiple sitemaps, no sitemap, case insensitive, whitespace trimming, empty robots)
- **Files Changed:**
  - Created: `/code/tools/provider-scraper/scraper/discovery.py` (232 lines)
  - Created: `/code/tools/provider-scraper/tests/test_discovery.py` (222 lines)
- **Test Results:** All 20 tests passed in 1.07s (2 warnings from crawl4ai's Pydantic v2 migration, not our code)
- **Review Findings:**
  - ❌ **INITIAL**: Silent exception swallowing violated "FAIL FAST" principle - bare `except Exception:` without logging
  - ❌ **INITIAL**: Code duplication in HTTP fetching logic (fetched sitemaps in multiple places)
  - ❌ **INITIAL**: Overly complex nested error handling in sitemap index detection
  - ❌ **INITIAL**: Missing tests for helper functions (`_extract_sitemaps_from_robots`)
  - ✅ **REFACTORED**: Added logging at appropriate levels (debug/info/warning/error) for observability
  - ✅ **REFACTORED**: Extracted `_fetch_sitemap_content()` and `_fetch_child_sitemaps()` helpers (DRY)
  - ✅ **REFACTORED**: Specific exception handling (`httpx.HTTPError` vs generic `Exception`)
  - ✅ **REFACTORED**: Added 6 tests for `_extract_sitemaps_from_robots()` (positive and negative paths)
  - ✅ **REFACTORED**: Simplified sitemap index detection from `any()` to `all()` - clearer intent
  - ✅ KISS/MINIMAL: Clean, focused implementation (232 lines including logging/error handling)
  - ✅ DRY: No duplication - HTTP fetching extracted to helpers
  - ✅ Tests comprehensive: Both positive and negative paths covered (20 tests total)
  - ✅ Follows codebase patterns: Matches crawler.py style (docstrings, type hints, imports)
  - ✅ FAIL FAST: Proper logging at all failure points, specific exceptions logged with context
- **Outcome:** SUCCESS - URL discovery module implemented, reviewed, refactored for quality

### Step 3
- **Implementation:** Created `/code/tools/provider-scraper/scraper/storage.py` with:
  - `CacheEntry` dataclass - filename, etag, content_hash, crawled_at fields
  - `DocsArchive.__init__(output_dir)` - Creates output directory, initializes zip_path/cache_path, loads cache
  - `DocsArchive._load_cache()` - Loads cache.json, returns empty dict if not exists, raises on invalid JSON
  - `DocsArchive._save_cache()` - Saves cache to cache.json with proper serialization
  - `DocsArchive._content_hash(content)` - Computes SHA256 hash, returns first 16 chars
  - `DocsArchive._safe_filename(url, topic)` - Generates safe filename from URL/topic, extracts path segment from URL, replaces special chars with hyphens, collapses multiple hyphens, strips leading/trailing hyphens, falls back to "index" for empty path, falls back to "page" if sanitization results in empty string
  - `DocsArchive.has_changed(url, etag, content)` - Returns True for new URLs, uses ETag comparison when available on both sides, falls back to content hash comparison
  - `DocsArchive.write(url, content, topic, etag)` - Writes markdown to ZIP (reads all, updates, writes all due to ZIP format limitation), updates cache, saves cache to disk, returns filename
  - `DocsArchive.read(url)` - Reads markdown from ZIP by URL lookup in cache, returns None if not found or ZIP missing
- **Tests:** Created `/code/tools/provider-scraper/tests/test_storage.py` with 37 unit tests covering:
  - Initialization: directory creation (1 test), path initialization (1 test), empty cache (1 test)
  - Cache load/save: JSON serialization (3 tests) - save format, load existing, invalid JSON raises
  - Content hashing: 16 char length (1 test), consistency (1 test), different content (1 test)
  - Safe filename: topic usage (1 test), URL extraction (1 test), special char replacement (1 test), hyphen collapsing (1 test), hyphen stripping (1 test), empty path fallback (1 test), special-chars-only topic fallback (1 test), alphanumeric preservation (1 test)
  - Change detection: new URL (1 test), ETag match (1 test), ETag mismatch (1 test), content hash fallback (1 test), content hash mismatch (1 test), ETag priority over content hash (1 test)
  - Write operations: ZIP creation (1 test), content storage (1 test), cache update (1 test), cache persistence (1 test), file update (1 test), preserve other files (1 test)
  - Read operations: missing URL (1 test), missing ZIP (1 test), existing content (1 test), missing file in ZIP (1 test), multiple files (1 test)
  - Integration: write-read roundtrip (1 test), incremental crawl simulation (1 test), cache persistence across instances (1 test)
- **Files Changed:**
  - Created: `/code/tools/provider-scraper/scraper/storage.py` (228 lines)
  - Created: `/code/tools/provider-scraper/tests/test_storage.py` (297 lines)
- **Test Results:** All 37 tests passed in 0.53s
- **Implementation Notes:**
  - ZIP format doesn't support in-place updates, so write() reads all files, updates the target, and writes a new ZIP
  - ETag comparison takes priority over content hash when available on both sides
  - Content hash uses first 16 chars of SHA256 for cache.json compactness
  - Logging added at debug/info/warning/error levels for observability
  - Safe filename generation extracts path segment (parts[3:]) to avoid using domain as filename
- **Review Findings:**
  - ✅ KISS/MINIMAL: Clean, focused implementation (228 lines) - no unnecessary complexity
  - ✅ DRY: No duplication found in codebase - content hashing and filename sanitization are unique to storage module
  - ✅ Tests comprehensive: Both positive and negative paths covered (37 tests total)
  - ✅ Follows codebase patterns: Matches crawler.py and discovery.py style (docstrings, type hints, logging, imports)
  - ⚠️ **INITIAL**: Missing test for edge case where topic contains only special characters (triggers "page" fallback)
  - ✅ **FIXED**: Added `test_safe_filename_handles_topic_with_only_special_chars` test to cover edge case (37 tests total now)
  - ✅ No architectural issues: Single responsibility, clear API, proper error handling, no silent failures
- **Outcome:** SUCCESS - ZIP storage and cache module implemented, reviewed, and tested with complete edge case coverage

### Step 4
- **Implementation:** Refactored `/code/tools/provider-scraper/scraper/base.py` to async with Crawl4AI integration:
  - Removed `httpx.Client` and context manager methods - now async-based
  - Added `docs_base_url` class attribute (optional, defaults to `provider_website`)
  - Integrated `DocsArchive` from storage.py - initialized in `__init__`
  - Changed `scrape_offerings()` to async abstract method
  - Added `discover_doc_urls()` - tries sitemap first, falls back to deep crawl (max_depth=2, max_pages=50)
  - Added `_filter_doc_urls()` - filters to docs/help/support/guide/faq/tutorial/knowledge patterns, subclass-overridable
  - Added `scrape_docs()` - crawls URLs, checks `archive.has_changed()`, extracts `fit_markdown` or `raw_markdown`, saves to ZIP with ETag/hash
  - Added `_extract_topic()` - extracts topic from page title (< 100 chars) or URL path segment
  - Changed `run()` to async - returns `(csv_path, docs_count)` instead of `(csv_path, list[md_paths])`
  - Removed old `scrape_docs()` abstract method (was returning `list[MarkdownDoc]`)
  - Uses `AsyncWebCrawler` with `DEFAULT_BROWSER_CONFIG` and `create_crawl_config()`
  - Logging at info/debug/warning/error levels for observability
- **Tests:** Created `/code/tools/provider-scraper/tests/test_base.py` with 28 unit tests covering:
  - Initialization: custom/default output dir (2 tests), provider_id generation (2 tests)
  - Doc URL discovery: docs_base_url usage (1 test), provider_website fallback (1 test), sitemap first (1 test), deep crawl fallback (1 test)
  - URL filtering: docs/help patterns (2 tests), multiple patterns (1 test), case insensitive (1 test), empty list (1 test)
  - Topic extraction: title usage (1 test), URL fallback (1 test), long title handling (1 test), root URL handling (1 test), path segment extraction (1 test)
  - Abstract method enforcement: scrape_offerings abstract (1 test)
  - Docs scraping: no URLs (1 test), crawl and write (1 test), skip unchanged (1 test), failed crawl (1 test), no markdown (1 test), raw markdown fallback (1 test), exception handling (1 test)
  - Full workflow: run() method (2 tests) - CSV writing and docs scraping
- **Files Changed:**
  - Modified: `/code/tools/provider-scraper/scraper/base.py` (173 lines, was 85 lines - removed old httpx sync code, added async Crawl4AI integration)
  - Created: `/code/tools/provider-scraper/tests/test_base.py` (421 lines)
- **Test Results:** All 28 tests passed in 1.26s (2 warnings from crawl4ai's Pydantic v2 migration, not our code). Full suite: 130 tests passed in 1.84s.
- **Review Findings:**
  - ✅ KISS/MINIMAL: Clean async implementation (173 lines) - all methods focused and single-purpose, no unnecessary complexity
  - ✅ DRY: No duplication - delegates to crawler.py, discovery.py, storage.py modules, all imports used
  - ✅ Tests comprehensive: Both positive and negative paths covered (28 tests, all unique assertions, no overlaps)
  - ✅ Follows codebase patterns: Matches crawler/discovery/storage style (docstrings, type hints, logging, error handling)
  - ✅ FAIL FAST: Proper logging at all failure points, errors logged with context, continues after errors to maximize crawl completion
  - ✅ Async migration complete: All sync httpx code removed, AsyncWebCrawler used, abstract method signature changed to async
  - ✅ No architectural issues: Clear separation of concerns, base class delegates to specialized modules
  - ✅ Integration correct: Uses crawler.py (DEFAULT_BROWSER_CONFIG, create_crawl_config), discovery.py (discover_sitemap, discover_via_crawl), storage.py (DocsArchive)
  - ✅ No simplifications needed: Code is already minimal and follows YAGNI/KISS principles
- **Outcome:** SUCCESS - Async base class implemented, reviewed, and tested with full Crawl4AI integration. Ready for Step 5.

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
