"""CLI for running provider scrapers."""

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


def main() -> None:
    """Run one or all scrapers."""
    output_base = Path(__file__).parent.parent / "output"

    # Get provider from args or run all
    providers = sys.argv[1:] if len(sys.argv) > 1 else list(SCRAPERS.keys())

    for provider in providers:
        if provider not in SCRAPERS:
            print(f"Unknown provider: {provider}")
            print(f"Available: {', '.join(SCRAPERS.keys())}")
            sys.exit(1)

        print(f"\n=== Scraping {provider} ===")
        scraper_cls = SCRAPERS[provider]
        output_dir = output_base / provider

        with scraper_cls(output_dir) as scraper:
            csv_path, md_paths = scraper.run()
            print(f"CSV: {csv_path} ({csv_path.stat().st_size} bytes)")
            print(f"Markdown files: {len(md_paths)}")
            for path in md_paths:
                size = path.stat().st_size
                status = "OK" if size <= 20480 else "OVER 20KB!"
                print(f"  {status}: {path.name} ({size} bytes)")

    print("\n=== Summary ===")
    total_offerings = 0
    total_docs = 0
    for provider in providers:
        csv_path = output_base / provider / "offerings.csv"
        if csv_path.exists():
            with csv_path.open() as f:
                count = sum(1 for _ in f) - 1  # subtract header
                total_offerings += count
                print(f"{provider}: {count} offerings")

        docs_dir = output_base / provider / "docs"
        if docs_dir.exists():
            doc_count = len(list(docs_dir.glob("*.md")))
            total_docs += doc_count

    print(f"\nTotal: {total_offerings} offerings, {total_docs} doc files")


if __name__ == "__main__":
    main()
