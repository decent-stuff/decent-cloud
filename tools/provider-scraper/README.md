# Provider Scraper

Scrapes hosting provider offerings and documentation for catalog seeding. Uses Crawl4AI for professional-grade web crawling with intelligent caching.

## Setup

```bash
cd tools/provider-scraper

# Create virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Install dependencies
pip install -e .

# Install Playwright browsers (required by Crawl4AI)
playwright install chromium
```

## Usage

```bash
# Scrape all providers
scrape

# Scrape specific provider
scrape hetzner
scrape contabo
scrape ovh

# Or run directly
python -m scraper.cli hetzner
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
# Install dev dependencies
pip install -e ".[dev]"

# Run tests
pytest

# Run linter
ruff check .

# Type check
pyright
```

## Architecture

- `crawler.py` - Crawl4AI wrapper with project defaults
- `discovery.py` - URL discovery via sitemap + BFS deep crawl
- `storage.py` - ZIP archive storage with ETag/content-hash caching
- `base.py` - Async base class for provider scrapers
- `providers/` - Provider-specific scrapers (Hetzner, Contabo, OVH)
