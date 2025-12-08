"""CLI for running provider scrapers."""

import asyncio
import logging
import sys
from pathlib import Path

from scraper.providers.contabo import ContaboScraper
from scraper.providers.hetzner import HetznerScraper
from scraper.providers.ovh import OvhScraper

SCRAPERS = {
    "hetzner": HetznerScraper,
    "contabo": ContaboScraper,
    "ovh": OvhScraper,
}

logger = logging.getLogger(__name__)


async def run_scraper(name: str, output_base: Path) -> tuple[int, int]:
    """Run single scraper and return counts.

    Args:
        name: Provider name (e.g., "hetzner")
        output_base: Base output directory

    Returns:
        Tuple of (offerings_count, docs_count)
    """
    print(f"\n=== Scraping {name} ===")
    scraper_cls = SCRAPERS[name]
    output_dir = output_base / name

    try:
        scraper = scraper_cls(output_dir=output_dir)
        csv_path, docs_count = await scraper.run()

        # Count offerings from CSV
        offerings_count = 0
        if csv_path.exists():
            with csv_path.open() as f:
                offerings_count = sum(1 for _ in f) - 1  # subtract header

        print(f"  Offerings: {offerings_count}")
        print(f"  Docs: {docs_count} new/changed")

        return offerings_count, docs_count

    except Exception as e:
        logger.error(f"Failed to scrape {name}: {e}")
        print(f"  ERROR: {e}")
        return 0, 0


def print_usage() -> None:
    """Print usage information."""
    print("Usage: uv run python3 -m scraper.cli [COMMAND|PROVIDER...]")
    print()
    print("Scrape hosting provider offerings and documentation.")
    print()
    print("Commands:")
    print("  setup    Install Playwright browsers (run once after install)")
    print("  help     Show this help message")
    print()
    print("Providers:")
    for name in SCRAPERS:
        print(f"  {name}")
    print()
    print("Examples:")
    print("  uv run python3 -m scraper.cli setup     # Install browsers (first time)")
    print("  uv run python3 -m scraper.cli           # Scrape all providers")
    print("  uv run python3 -m scraper.cli hetzner   # Scrape Hetzner only")


def run_setup() -> None:
    """Install Playwright browsers required by Crawl4AI."""
    import subprocess

    print("Installing Playwright browsers...")
    result = subprocess.run(
        [sys.executable, "-m", "playwright", "install", "chromium"],
        capture_output=False,
    )
    if result.returncode == 0:
        print("\nSetup complete! You can now run: uv run python3 -m scraper.cli")
    else:
        print("\nSetup failed. Try running manually: playwright install chromium")
        sys.exit(1)


async def main() -> None:
    """Run one or all scrapers and print summary."""
    # Handle commands
    if len(sys.argv) > 1:
        cmd = sys.argv[1]
        if cmd in ("-h", "--help", "help"):
            print_usage()
            return
        if cmd == "setup":
            run_setup()
            return

    logging.basicConfig(level=logging.INFO)
    output_base = Path(__file__).parent.parent / "output"

    # Get provider from args or run all
    providers = sys.argv[1:] if len(sys.argv) > 1 else list(SCRAPERS.keys())

    # Validate providers
    for provider in providers:
        if provider not in SCRAPERS:
            print(f"Unknown provider: {provider}")
            print(f"Available: {', '.join(SCRAPERS.keys())}")
            print()
            print("Run with -h for help.")
            sys.exit(1)

    # Run scrapers
    total_offerings = 0
    total_docs = 0

    for provider in providers:
        offerings_count, docs_count = await run_scraper(provider, output_base)
        total_offerings += offerings_count
        total_docs += docs_count

    # Print summary
    print("\n=== Summary ===")
    print(f"Total: {total_offerings} offerings, {total_docs} docs")


def cli() -> None:
    """Entry point for CLI."""
    asyncio.run(main())


if __name__ == "__main__":
    cli()
