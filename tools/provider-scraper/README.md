# Provider Scraper

Scrapes hosting provider offerings and documentation for catalog seeding. Uses Crawl4AI for professional-grade web crawling with intelligent caching.

## Quick Start

```bash
cd tools/provider-scraper

# First time: install Playwright browsers
uv run python3 -m scraper.cli setup

# Scrape all providers
uv run python3 -m scraper.cli

# Scrape specific provider
uv run python3 -m scraper.cli hetzner
```

## Usage

```
Usage: uv run python3 -m scraper.cli [COMMAND|PROVIDER...]

Commands:
  setup    Install Playwright browsers (run once after install)
  help     Show help message

Providers:
  hetzner
  contabo
  ovh

Examples:
  uv run python3 -m scraper.cli setup     # Install browsers (first time)
  uv run python3 -m scraper.cli           # Scrape all providers
  uv run python3 -m scraper.cli hetzner   # Scrape Hetzner only
```

## Output

```
output/
├── hetzner/
│   ├── offerings.csv    # Product catalog (VPS plans, pricing, specs)
│   ├── docs.zip         # Documentation markdown files
│   └── cache.json       # ETag/hash cache for incremental crawls
├── contabo/
│   └── ...
└── ovh/
    └── ...
```

## Development

```bash
# Run tests
uv run pytest

# Lint
uv run ruff check .

# Type check
uv run pyright
```

## Architecture

- `crawler.py` - Crawl4AI wrapper with project defaults
- `discovery.py` - URL discovery via sitemap + BFS deep crawl
- `storage.py` - ZIP archive storage with ETag/content-hash caching
- `base.py` - Async base class for provider scrapers
- `providers/` - Provider-specific scrapers (Hetzner, Contabo, OVH)
