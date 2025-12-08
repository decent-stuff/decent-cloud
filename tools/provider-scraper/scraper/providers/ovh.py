"""OVH Cloud scraper using public catalog API."""

import re

import httpx

from scraper.base import BaseScraper
from scraper.models import Offering


class OvhScrapeError(Exception):
    """Raised when OVH scraping fails."""


class OvhScraper(BaseScraper):
    """Scraper for OVH Cloud offerings using public catalog API."""

    provider_name = "OVH"
    provider_website = "https://www.ovhcloud.com"
    docs_base_url = "https://help.ovhcloud.com"

    # OVH public catalog API (no auth required)
    API_BASE = "https://eu.api.ovh.com/1.0"

    # Subsidiaries to fetch pricing from (affects currency and availability)
    SUBSIDIARIES = ["FR", "DE", "GB", "US"]

    async def scrape_offerings(self) -> list[Offering]:
        """Fetch offerings from OVH public catalog API.

        Raises:
            OvhScrapeError: If API request fails.
        """
        async with httpx.AsyncClient(timeout=30.0) as client:
            all_offerings = []

            # Fetch VPS catalog
            for subsidiary in self.SUBSIDIARIES:
                try:
                    vps_offerings = await self._fetch_vps_catalog(client, subsidiary)
                    all_offerings.extend(vps_offerings)
                except OvhScrapeError as e:
                    # Log but continue with other subsidiaries
                    import logging
                    logging.warning(f"Failed to fetch VPS catalog for {subsidiary}: {e}")

        if not all_offerings:
            raise OvhScrapeError("No offerings fetched from any OVH subsidiary")

        return all_offerings

    async def _fetch_vps_catalog(
        self, client: httpx.AsyncClient, subsidiary: str
    ) -> list[Offering]:
        """Fetch VPS catalog for a specific subsidiary."""
        url = f"{self.API_BASE}/order/catalog/public/vps"
        response = await client.get(url, params={"ovhSubsidiary": subsidiary})

        if response.status_code == 429:
            raise OvhScrapeError(f"Rate limited by OVH API ({subsidiary})")
        if response.status_code != 200:
            raise OvhScrapeError(
                f"OVH API error for {subsidiary}: {response.status_code} - {response.text[:200]}"
            )

        data = response.json()
        plans = data.get("plans", [])

        if not plans:
            raise OvhScrapeError(f"No VPS plans in catalog for {subsidiary}")

        return self._parse_plans(plans, subsidiary)

    def _parse_plans(self, plans: list[dict], subsidiary: str) -> list[Offering]:
        """Parse VPS plans into Offering objects."""
        offerings = []
        seen_plans = set()  # Dedupe by base plan code

        for plan in plans:
            plan_code = plan.get("planCode", "")
            if not plan_code:
                continue

            # Extract base plan info from plan code
            # Format: vps-{tier}-{vcpu}-{ram}-{disk}-vps-{year}-{model}-{options}
            # Example: vps-essential-2-4-160-vps-2025-model3-degressivity12-10percent
            base_match = re.match(
                r"vps-(\w+)-(\d+)-(\d+)-(\d+)", plan_code
            )
            if not base_match:
                continue

            tier, vcpu, ram, disk = base_match.groups()

            # Create a dedup key (avoid duplicate plans with different commitment options)
            dedup_key = f"{tier}-{vcpu}-{ram}-{disk}-{subsidiary}"
            if dedup_key in seen_plans:
                continue
            seen_plans.add(dedup_key)

            # Find monthly renewal price
            monthly_price = self._extract_monthly_price(plan)
            if monthly_price is None:
                continue

            # Determine currency based on subsidiary
            currency = "EUR" if subsidiary in ("FR", "DE") else "GBP" if subsidiary == "GB" else "USD"

            # Determine datacenter info
            country, city = self._subsidiary_to_location(subsidiary)

            offering = Offering(
                offering_id=f"ovh-vps-{tier}-{vcpu}-{ram}-{disk}-{subsidiary.lower()}",
                offer_name=f"OVH VPS {tier.capitalize()} {vcpu}vCPU/{ram}GB - {city}",
                description=f"OVH VPS {tier} with {vcpu} vCPUs, {ram}GB RAM, {disk}GB SSD",
                product_page_url="https://www.ovhcloud.com/en/vps/",
                currency=currency,
                monthly_price=monthly_price,
                setup_fee=0.0,
                visibility="public",
                product_type="compute",
                virtualization_type="kvm",
                billing_interval="monthly",
                stock_status="in_stock",
                datacenter_country=country,
                datacenter_city=city,
                processor_cores=int(vcpu),
                memory_amount=int(ram),
                total_ssd_capacity=int(disk),
                unmetered_bandwidth=True,
                features="DDoS Protection,IPv4,IPv6,Daily Backup",
                operating_systems="Ubuntu,Debian,CentOS,Rocky,AlmaLinux,Fedora,Windows",
            )
            offerings.append(offering)

        return offerings

    def _extract_monthly_price(self, plan: dict) -> float | None:
        """Extract monthly renewal price from plan pricings."""
        for pricing in plan.get("pricings", []):
            capacities = pricing.get("capacities", [])
            interval_unit = pricing.get("intervalUnit", "")

            # Look for monthly renewal pricing
            if "renew" in capacities and interval_unit == "month":
                # Price is in micro-currency (divide by 10^8 to get actual price)
                price_raw = pricing.get("price", 0)
                if price_raw:
                    return price_raw / 100_000_000

        return None

    def _subsidiary_to_location(self, subsidiary: str) -> tuple[str, str]:
        """Map subsidiary code to country and city."""
        mapping = {
            "FR": ("FR", "Gravelines"),
            "DE": ("DE", "Frankfurt"),
            "GB": ("GB", "London"),
            "US": ("US", "Vint Hill"),
        }
        return mapping.get(subsidiary, (subsidiary, subsidiary))


async def main() -> None:
    """Run the OVH scraper."""
    from pathlib import Path

    output_dir = Path(__file__).parent.parent.parent / "output" / "ovh"

    scraper = OvhScraper(output_dir)
    csv_path, docs_count = await scraper.run()
    print(f"CSV written to: {csv_path}")
    print(f"Docs written: {docs_count}")


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
