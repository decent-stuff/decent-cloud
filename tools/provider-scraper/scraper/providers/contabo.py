"""Contabo scraper - requires web scraping (no public API for pricing)."""

import re

import httpx
from bs4 import BeautifulSoup

from scraper.base import BaseScraper
from scraper.models import Offering


class ContaboScrapeError(Exception):
    """Raised when Contabo scraping fails."""


class ContaboScraper(BaseScraper):
    """Scraper for Contabo offerings via web scraping."""

    provider_name = "Contabo"
    provider_website = "https://contabo.com"
    docs_base_url = "https://docs.contabo.com"

    VPS_URL = "https://contabo.com/en/vps/"
    VDS_URL = "https://contabo.com/en/vds/"

    # Contabo locations (these are stable, but we verify connectivity)
    LOCATIONS = [
        {"code": "eu-de-1", "city": "Nuremberg", "country": "DE"},
        {"code": "eu-de-2", "city": "Dusseldorf", "country": "DE"},
        {"code": "us-east-1", "city": "New York", "country": "US"},
        {"code": "us-central-1", "city": "St. Louis", "country": "US"},
        {"code": "us-west-1", "city": "Seattle", "country": "US"},
        {"code": "ap-sin-1", "city": "Singapore", "country": "SG"},
        {"code": "ap-syd-1", "city": "Sydney", "country": "AU"},
        {"code": "ap-tky-1", "city": "Tokyo", "country": "JP"},
        {"code": "uk-lon-1", "city": "London", "country": "GB"},
    ]

    async def scrape_offerings(self) -> list[Offering]:
        """Scrape offerings from Contabo website.

        Raises:
            ContaboScrapeError: If web scraping fails.
        """
        async with httpx.AsyncClient(timeout=30.0, follow_redirects=True) as client:
            vps_plans = await self._scrape_plans(client, self.VPS_URL, "compute")
            vds_plans = await self._scrape_plans(client, self.VDS_URL, "dedicated")

        all_plans = vps_plans + vds_plans
        if not all_plans:
            raise ContaboScrapeError("No plans found on Contabo website")

        return self._build_offerings(all_plans)

    async def _scrape_plans(
        self, client: httpx.AsyncClient, url: str, product_type: str
    ) -> list[dict]:
        """Scrape plans from a Contabo pricing page."""
        response = await client.get(url)

        if response.status_code == 429:
            raise ContaboScrapeError(f"Rate limited by Contabo - try again later ({url})")
        if response.status_code != 200:
            raise ContaboScrapeError(
                f"Failed to fetch {url}: HTTP {response.status_code}"
            )

        soup = BeautifulSoup(response.text, "html.parser")
        plans = []

        # Look for pricing cards - Contabo uses various class patterns
        # Try to find plan containers
        plan_cards = soup.select(".pricing-card, .plan-card, [class*='pricing'], [class*='plan']")

        if not plan_cards:
            # Try alternative: look for price elements
            price_elements = soup.find_all(string=re.compile(r"€\s*\d+[.,]\d{2}"))
            if not price_elements:
                raise ContaboScrapeError(
                    f"Could not parse pricing structure from {url} - page format may have changed"
                )

        # Parse pricing information from the page
        for card in plan_cards:
            plan = self._parse_plan_card(card, product_type)
            if plan:
                plans.append(plan)

        # If card parsing didn't work, try structured data
        if not plans:
            plans = self._extract_from_structured_data(soup, product_type)

        return plans

    def _parse_plan_card(self, card, product_type: str) -> dict | None:
        """Parse a single plan card element."""
        try:
            # Look for plan name
            name_elem = card.select_one("h2, h3, .plan-name, .title")
            if not name_elem:
                return None
            name = name_elem.get_text(strip=True)

            # Look for price
            price_elem = card.select_one(".price, [class*='price']")
            if not price_elem:
                return None
            price_text = price_elem.get_text(strip=True)
            price_match = re.search(r"€?\s*(\d+)[.,](\d{2})", price_text)
            if not price_match:
                return None
            price = float(f"{price_match.group(1)}.{price_match.group(2)}")

            # Look for specs (vCPU, RAM, storage)
            specs_text = card.get_text()
            vcpu_match = re.search(r"(\d+)\s*(?:vCPU|CPU|Core)", specs_text, re.I)
            ram_match = re.search(r"(\d+)\s*GB\s*(?:RAM|Memory)", specs_text, re.I)
            storage_match = re.search(r"(\d+)\s*GB\s*(?:SSD|NVMe|Storage)", specs_text, re.I)

            return {
                "name": name,
                "price_eur": price,
                "vcpu": int(vcpu_match.group(1)) if vcpu_match else 0,
                "ram": int(ram_match.group(1)) if ram_match else 0,
                "storage": int(storage_match.group(1)) if storage_match else 0,
                "product_type": product_type,
            }
        except Exception:
            return None

    def _extract_from_structured_data(self, soup: BeautifulSoup, product_type: str) -> list[dict]:
        """Try to extract plan data from JSON-LD or other structured data."""
        plans = []

        # Look for JSON-LD
        for script in soup.find_all("script", type="application/ld+json"):
            try:
                import json
                data = json.loads(script.string)
                if isinstance(data, dict) and data.get("@type") == "Product":
                    offers = data.get("offers", {})
                    if isinstance(offers, dict):
                        offers = [offers]
                    for offer in offers:
                        price = offer.get("price")
                        if price:
                            plans.append({
                                "name": data.get("name", "Unknown"),
                                "price_eur": float(price),
                                "vcpu": 0,
                                "ram": 0,
                                "storage": 0,
                                "product_type": product_type,
                            })
            except Exception:
                continue

        return plans

    def _build_offerings(self, plans: list[dict]) -> list[Offering]:
        """Build Offering objects from scraped plans."""
        offerings = []

        for plan in plans:
            for loc in self.LOCATIONS:
                offering = Offering(
                    offering_id=f"contabo-{plan['name'].lower().replace(' ', '-')}-{loc['code']}",
                    offer_name=f"Contabo {plan['name']} - {loc['city']}",
                    description=f"Contabo {plan['name']} in {loc['city']}, {loc['country']}",
                    product_page_url=self.VPS_URL if plan["product_type"] == "compute" else self.VDS_URL,
                    currency="EUR",
                    monthly_price=plan["price_eur"],
                    setup_fee=0.0,
                    visibility="public",
                    product_type=plan["product_type"],
                    virtualization_type="kvm",
                    billing_interval="monthly",
                    stock_status="in_stock",
                    datacenter_country=loc["country"],
                    datacenter_city=loc["city"],
                    processor_cores=plan["vcpu"] if plan["vcpu"] > 0 else None,
                    memory_amount=plan["ram"] if plan["ram"] > 0 else None,
                    total_ssd_capacity=plan["storage"] if plan["storage"] > 0 else None,
                    unmetered_bandwidth=True,
                    features="DDoS Protection,IPv4,IPv6,Snapshots",
                    operating_systems="Ubuntu,Debian,CentOS,Rocky,AlmaLinux,Windows",
                )
                offerings.append(offering)

        return offerings


async def main() -> None:
    """Run the Contabo scraper."""
    from pathlib import Path

    output_dir = Path(__file__).parent.parent.parent / "output" / "contabo"

    scraper = ContaboScraper(output_dir)
    csv_path, docs_count = await scraper.run()
    print(f"CSV written to: {csv_path}")
    print(f"Docs written: {docs_count}")


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
