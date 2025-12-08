"""Hetzner Cloud scraper using live API data."""

import os

import httpx

from scraper.base import BaseScraper
from scraper.models import Offering


class HetznerScrapeError(Exception):
    """Raised when Hetzner scraping fails."""


class HetznerScraper(BaseScraper):
    """Scraper for Hetzner Cloud offerings using the Hetzner Cloud API."""

    provider_name = "Hetzner"
    provider_website = "https://www.hetzner.com"
    # Use docs.hetzner.cloud (accessible) instead of docs.hetzner.com (blocked with 429)
    docs_base_url = "https://docs.hetzner.cloud"

    # Q&A generation sources
    changelog_url = "https://docs.hetzner.cloud/changelog"
    faq_urls = [
        "https://docs.hetzner.cloud/",  # API overview
        "https://docs.hetzner.cloud/reference/cloud",  # Cloud API reference
    ]
    # GitHub tutorials repo (most reliable source, no bot protection)
    github_docs_repo = "hetzneronline/community-content"
    github_docs_path = "tutorials"

    API_BASE = "https://api.hetzner.cloud/v1"

    # Explicit doc URLs (docs.hetzner.cloud is a React SPA, no sitemap, deep crawl fails)
    # Only include pages with distinct content (some paths redirect to overview page)
    DOC_URLS = [
        "https://docs.hetzner.cloud/",  # API overview
        "https://docs.hetzner.cloud/reference/cloud",  # Cloud API reference
        "https://docs.hetzner.cloud/changelog",  # Changelog with updates
    ]

    async def discover_doc_urls(self) -> list[str]:
        """Return hardcoded doc URLs since docs.hetzner.cloud is a React SPA with no sitemap."""
        return self.DOC_URLS

    async def scrape_offerings(self) -> list[Offering]:
        """Fetch offerings from Hetzner Cloud API.

        Requires HETZNER_API_TOKEN environment variable.

        Raises:
            HetznerScrapeError: If API token is missing or API request fails.
        """
        api_token = os.environ.get("HETZNER_API_TOKEN")
        if not api_token:
            raise HetznerScrapeError(
                "HETZNER_API_TOKEN environment variable is required. "
                "Create one at https://console.hetzner.cloud/ → Security → API Tokens"
            )

        headers = {"Authorization": f"Bearer {api_token}"}

        async with httpx.AsyncClient(timeout=30.0) as client:
            # Fetch server types
            server_types = await self._fetch_server_types(client, headers)
            # Fetch locations
            locations = await self._fetch_locations(client, headers)

        return self._build_offerings(server_types, locations)

    async def _fetch_server_types(self, client: httpx.AsyncClient, headers: dict) -> list[dict]:
        """Fetch all server types from API."""
        all_types = []
        page = 1

        while True:
            response = await client.get(
                f"{self.API_BASE}/server_types",
                headers=headers,
                params={"page": page, "per_page": 50},
            )

            if response.status_code == 401:
                raise HetznerScrapeError("Invalid HETZNER_API_TOKEN - authentication failed")
            if response.status_code == 429:
                raise HetznerScrapeError("Rate limited by Hetzner API - try again later")
            if response.status_code != 200:
                raise HetznerScrapeError(
                    f"Hetzner API error: {response.status_code} - {response.text}"
                )

            data = response.json()
            all_types.extend(data.get("server_types", []))

            # Check pagination
            meta = data.get("meta", {}).get("pagination", {})
            if page >= meta.get("last_page", 1):
                break
            page += 1

        if not all_types:
            raise HetznerScrapeError("No server types returned from Hetzner API")

        return all_types

    async def _fetch_locations(self, client: httpx.AsyncClient, headers: dict) -> list[dict]:
        """Fetch all locations from API."""
        response = await client.get(f"{self.API_BASE}/locations", headers=headers)

        if response.status_code != 200:
            raise HetznerScrapeError(
                f"Failed to fetch locations: {response.status_code} - {response.text}"
            )

        data = response.json()
        locations = data.get("locations", [])

        if not locations:
            raise HetznerScrapeError("No locations returned from Hetzner API")

        return locations

    def _build_offerings(self, server_types: list[dict], locations: list[dict]) -> list[Offering]:
        """Build Offering objects from API data."""
        offerings = []
        location_map = {loc["name"]: loc for loc in locations}

        for st in server_types:
            if st.get("deprecated"):
                continue

            for price_info in st.get("prices", []):
                loc_name = price_info.get("location")
                loc = location_map.get(loc_name, {})

                # Parse monthly price (gross)
                price_monthly = price_info.get("price_monthly", {})
                price_str = price_monthly.get("gross", "0")
                try:
                    price = float(price_str)
                except ValueError:
                    price = 0.0

                # Determine product type
                cpu_type = st.get("cpu_type", "shared")
                product_type = "dedicated" if cpu_type == "dedicated" else "compute"

                offering = Offering(
                    offering_id=f"hetzner-{st['name']}-{loc_name}",
                    offer_name=f"Hetzner Cloud {st['name']} - {loc.get('city', loc_name)}",
                    description=st.get("description", ""),
                    product_page_url="https://www.hetzner.com/cloud",
                    currency="EUR",
                    monthly_price=price,
                    setup_fee=0.0,
                    visibility="public",
                    product_type=product_type,
                    virtualization_type="kvm",
                    billing_interval="monthly",
                    stock_status="in_stock",
                    datacenter_country=loc.get("country", ""),
                    datacenter_city=loc.get("city", ""),
                    processor_cores=st.get("cores"),
                    memory_amount=int(st.get("memory", 0)),
                    total_ssd_capacity=st.get("disk"),
                    uplink_speed=1000,  # 1 Gbps standard
                    traffic=st.get("included_traffic", 0) // (1024 * 1024 * 1024),  # bytes to GB
                    unmetered_bandwidth=False,
                    features="IPv4,IPv6,Snapshots,Backups,Firewall",
                    operating_systems="Ubuntu,Debian,Fedora,Rocky,AlmaLinux,CentOS",
                )
                offerings.append(offering)

        if not offerings:
            raise HetznerScrapeError("No offerings could be built from API data")

        return offerings


async def main() -> None:
    """Run the Hetzner scraper."""
    from pathlib import Path

    output_dir = Path(__file__).parent.parent.parent / "output" / "hetzner"

    scraper = HetznerScraper(output_dir)
    csv_path, docs_count = await scraper.run()
    print(f"CSV written to: {csv_path}")
    print(f"Docs written: {docs_count}")


if __name__ == "__main__":
    import asyncio

    asyncio.run(main())
