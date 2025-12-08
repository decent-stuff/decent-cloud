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


async def run_scraper(
    name: str,
    output_base: Path,
    skip_offerings: bool = False,
    skip_docs: bool = False,
    keep_local: bool = False,
    generate_qa: bool = False,
    force_qa: bool = False,
) -> tuple[int, int, int, bool]:
    """Run single scraper and return counts.

    Args:
        name: Provider name (e.g., "hetzner")
        output_base: Base output directory
        skip_offerings: If True, continue on offerings failure
        skip_docs: If True, continue on docs failure
        keep_local: If True, keep local docs/ directory for troubleshooting
        generate_qa: If True, generate Q&A content using LLM
        force_qa: If True, force Q&A regeneration even if no changes detected

    Returns:
        Tuple of (offerings_count, docs_count, qa_count, success)
    """
    print(f"\n=== Scraping {name} ===")
    scraper_cls = SCRAPERS[name]
    output_dir = output_base / name

    try:
        scraper = scraper_cls(output_dir=output_dir)
        csv_path, docs_count = await scraper.run(
            skip_offerings=skip_offerings,
            skip_docs=skip_docs,
            keep_local=keep_local,
        )

        # Count offerings from CSV
        offerings_count = 0
        offerings = []
        if csv_path and csv_path.exists():
            with csv_path.open() as f:
                offerings_count = sum(1 for _ in f) - 1  # subtract header

            # Re-scrape offerings for Q&A generation (already cached by API)
            if generate_qa:
                offerings = await scraper.scrape_offerings()

        print(f"  Offerings: {offerings_count}")
        print(f"  Docs: {docs_count} new/changed")

        # Generate Q&A if requested
        qa_count = 0
        if generate_qa and offerings:
            try:
                qa_path = await scraper.generate_qa(offerings, force=force_qa)
                if qa_path and qa_path.exists():
                    import json
                    qa_data = json.loads(qa_path.read_text())
                    qa_count = len(qa_data)
                    print(f"  Q&A: {qa_count} pairs generated")
                else:
                    print("  Q&A: skipped (no changes)")
            except Exception as e:
                logger.warning(f"Q&A generation failed: {e}")
                print(f"  Q&A: failed ({e})")

        return offerings_count, docs_count, qa_count, True

    except Exception as e:
        logger.error(f"Failed to scrape {name}: {e}")
        print(f"  ERROR: {e}")
        return 0, 0, 0, False


def print_usage() -> None:
    """Print usage information."""
    print("Usage: uv run python3 -m scraper.cli [OPTIONS] [PROVIDER...]")
    print()
    print("Scrape hosting provider offerings and documentation.")
    print()
    print("Options:")
    print("  --skip-offerings    Continue if offerings scrape fails")
    print("  --skip-docs         Continue if docs scrape fails")
    print("  --skip-failures     Continue on any failure (same as --skip-offerings --skip-docs)")
    print("  --keep-local        Keep local docs/ directory (don't delete after zipping)")
    print("  --generate-qa       Generate Q&A content using LLM (requires ANTHROPIC_API_KEY)")
    print("  --force-qa          Force Q&A regeneration even if no changes detected")
    print()
    print("Commands:")
    print("  setup    Install Playwright browsers (run once after install)")
    print("  help     Show this help message")
    print()
    print("Providers:")
    for name in SCRAPERS:
        print(f"  {name}")
    print()
    print("Environment variables:")
    print("  HETZNER_API_TOKEN    Required for Hetzner (create at console.hetzner.cloud)")
    print("  ANTHROPIC_API_KEY    Required for Q&A generation")
    print("  ANTHROPIC_BASE_URL   Optional custom API base URL")
    print("  ANTHROPIC_MODEL      Optional model override (default: claude-sonnet-4-20250514)")
    print()
    print("Examples:")
    print("  uv run python3 -m scraper.cli setup              # Install browsers (first time)")
    print("  uv run python3 -m scraper.cli                    # Scrape all providers")
    print("  uv run python3 -m scraper.cli hetzner            # Scrape Hetzner only")
    print("  uv run python3 -m scraper.cli --skip-docs ovh    # Skip docs failures for OVH")
    print("  uv run python3 -m scraper.cli --keep-local ovh   # Keep docs/ for troubleshooting")
    print("  uv run python3 -m scraper.cli --generate-qa hetzner  # Generate Q&A for Hetzner")


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


def parse_args(args: list[str]) -> tuple[list[str], bool, bool, bool, bool, bool]:
    """Parse command line arguments.

    Returns:
        Tuple of (providers, skip_offerings, skip_docs, keep_local, generate_qa, force_qa)
    """
    providers = []
    skip_offerings = False
    skip_docs = False
    keep_local = False
    generate_qa = False
    force_qa = False

    for arg in args:
        if arg == "--skip-offerings":
            skip_offerings = True
        elif arg == "--skip-docs":
            skip_docs = True
        elif arg == "--skip-failures":
            skip_offerings = True
            skip_docs = True
        elif arg == "--keep-local":
            keep_local = True
        elif arg == "--generate-qa":
            generate_qa = True
        elif arg == "--force-qa":
            force_qa = True
            generate_qa = True  # --force-qa implies --generate-qa
        elif not arg.startswith("-"):
            providers.append(arg)

    return providers, skip_offerings, skip_docs, keep_local, generate_qa, force_qa


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

    # Parse arguments
    providers, skip_offerings, skip_docs, keep_local, generate_qa, force_qa = parse_args(
        sys.argv[1:]
    )

    # Default to all providers if none specified
    if not providers:
        providers = list(SCRAPERS.keys())

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
    total_qa = 0
    failed_providers = []

    for provider in providers:
        offerings_count, docs_count, qa_count, success = await run_scraper(
            provider, output_base, skip_offerings, skip_docs, keep_local, generate_qa, force_qa
        )
        total_offerings += offerings_count
        total_docs += docs_count
        total_qa += qa_count
        if not success:
            failed_providers.append(provider)

    # Print summary
    print("\n=== Summary ===")
    summary = f"Total: {total_offerings} offerings, {total_docs} docs"
    if generate_qa:
        summary += f", {total_qa} Q&A pairs"
    print(summary)

    if failed_providers:
        print(f"Failed: {', '.join(failed_providers)}")
        sys.exit(1)


def cli() -> None:
    """Entry point for CLI."""
    asyncio.run(main())


if __name__ == "__main__":
    cli()
