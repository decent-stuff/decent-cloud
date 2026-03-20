# PROVIDER-SCRAPER KNOWLEDGE BASE

## OVERVIEW
`repo/tools/provider-scraper/` is a Python tool that crawls hosting providers, extracts catalog/docs data, and writes normalized output for seeding.

## STRUCTURE
```text
provider-scraper/
|- scraper/        # crawl, discovery, storage, provider implementations
|- tests/          # pytest suite
|- output/         # generated scrape artifacts
`- pyproject.toml  # uv/pytest/ruff/pyright config
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| CLI entry | `scraper/cli.py` | `uv run python3 -m scraper.cli ...` |
| Crawl defaults | `scraper/crawler.py` | Crawl4AI wrapper and config builders |
| Provider discovery | `scraper/discovery.py` | Sitemap + crawl expansion |
| Output persistence | `scraper/storage.py`, `scraper/csv_writer.py` | Docs archive + CSV emission |
| Provider-specific code | `scraper/providers/` | Hetzner, Contabo, OVH |
| Tests | `tests/` | pytest-based validation |

## CONVENTIONS
- Run through `uv`; this tool already declares its own Python project config.
- `output/` is generated data, not hand-maintained source.
- Public package exports are centralized in `scraper/__init__.py`.
- Keep provider-specific behavior in `scraper/providers/`; shared crawl/storage logic belongs in top-level modules.

## ANTI-PATTERNS
- Hand-editing generated scrape output as if it were source code.
- Duplicating provider-specific parsing in shared modules.
- Bypassing `uv run` and local project config during tests/linting.

## COMMANDS
```bash
uv run python3 -m scraper.cli setup
uv run python3 -m scraper.cli
uv run pytest
uv run ruff check .
uv run pyright
```

## NOTES
- This tool is first-party, but its outputs and caches should stay out of higher-level architecture docs.
