"""Hetzner Cloud scraper."""

from typing import TypedDict

from scraper.base import BaseScraper
from scraper.markdown import MarkdownDoc, html_to_markdown
from scraper.models import Offering


class HetznerPlan(TypedDict):
    """Type for Hetzner plan data."""

    id: str
    name: str
    vcpu: int
    ram: int
    ssd: int
    price_eur: float
    type: str


class HetznerLocation(TypedDict):
    """Type for Hetzner location data."""

    code: str
    city: str
    country: str
    traffic_tb: int


# Hetzner Cloud server plans with current pricing (EUR)
# Data from https://www.hetzner.com/cloud/ as of Dec 2025
# CAX = ARM (Ampere), CX = Shared Intel/AMD, CPX = Shared Performance, CCX = Dedicated

HETZNER_PLANS: list[HetznerPlan] = [
    # Shared vCPU - Cost Optimized (CX series - Intel/AMD)
    {"id": "cx22", "name": "CX22", "vcpu": 2, "ram": 4, "ssd": 40, "price_eur": 3.79, "type": "shared"},
    {"id": "cx32", "name": "CX32", "vcpu": 4, "ram": 8, "ssd": 80, "price_eur": 7.59, "type": "shared"},
    {"id": "cx42", "name": "CX42", "vcpu": 8, "ram": 16, "ssd": 160, "price_eur": 14.99, "type": "shared"},
    {"id": "cx52", "name": "CX52", "vcpu": 16, "ram": 32, "ssd": 320, "price_eur": 29.99, "type": "shared"},
    # Shared vCPU - ARM (CAX series - Ampere)
    {"id": "cax11", "name": "CAX11", "vcpu": 2, "ram": 4, "ssd": 40, "price_eur": 3.79, "type": "shared"},
    {"id": "cax21", "name": "CAX21", "vcpu": 4, "ram": 8, "ssd": 80, "price_eur": 6.49, "type": "shared"},
    {"id": "cax31", "name": "CAX31", "vcpu": 8, "ram": 16, "ssd": 160, "price_eur": 12.49, "type": "shared"},
    {"id": "cax41", "name": "CAX41", "vcpu": 16, "ram": 32, "ssd": 320, "price_eur": 24.49, "type": "shared"},
    # Shared vCPU - Performance (CPX series)
    {"id": "cpx11", "name": "CPX11", "vcpu": 2, "ram": 2, "ssd": 40, "price_eur": 4.99, "type": "shared"},
    {"id": "cpx21", "name": "CPX21", "vcpu": 3, "ram": 4, "ssd": 80, "price_eur": 9.49, "type": "shared"},
    {"id": "cpx31", "name": "CPX31", "vcpu": 4, "ram": 8, "ssd": 160, "price_eur": 16.49, "type": "shared"},
    {"id": "cpx41", "name": "CPX41", "vcpu": 8, "ram": 16, "ssd": 240, "price_eur": 30.49, "type": "shared"},
    {"id": "cpx51", "name": "CPX51", "vcpu": 16, "ram": 32, "ssd": 360, "price_eur": 60.49, "type": "shared"},
    # Dedicated vCPU (CCX series)
    {"id": "ccx13", "name": "CCX13", "vcpu": 2, "ram": 8, "ssd": 80, "price_eur": 12.49, "type": "dedicated"},
    {"id": "ccx23", "name": "CCX23", "vcpu": 4, "ram": 16, "ssd": 160, "price_eur": 24.49, "type": "dedicated"},
    {"id": "ccx33", "name": "CCX33", "vcpu": 8, "ram": 32, "ssd": 240, "price_eur": 48.49, "type": "dedicated"},
    {"id": "ccx43", "name": "CCX43", "vcpu": 16, "ram": 64, "ssd": 360, "price_eur": 96.49, "type": "dedicated"},
    {"id": "ccx53", "name": "CCX53", "vcpu": 32, "ram": 128, "ssd": 600, "price_eur": 192.49, "type": "dedicated"},
    {"id": "ccx63", "name": "CCX63", "vcpu": 48, "ram": 192, "ssd": 960, "price_eur": 288.49, "type": "dedicated"},
]

# Hetzner datacenter locations
HETZNER_LOCATIONS: list[HetznerLocation] = [
    {"code": "nbg1", "city": "Nuremberg", "country": "DE", "traffic_tb": 20},
    {"code": "fsn1", "city": "Falkenstein", "country": "DE", "traffic_tb": 20},
    {"code": "hel1", "city": "Helsinki", "country": "FI", "traffic_tb": 20},
    {"code": "ash", "city": "Ashburn", "country": "US", "traffic_tb": 1},
    {"code": "hil", "city": "Hillsboro", "country": "US", "traffic_tb": 1},
    {"code": "sin", "city": "Singapore", "country": "SG", "traffic_tb": 1},
]

# EUR to USD conversion (approximate)
EUR_TO_USD = 1.10


class HetznerScraper(BaseScraper):
    """Scraper for Hetzner Cloud offerings."""

    provider_name = "Hetzner"
    provider_website = "https://www.hetzner.com"

    def scrape_offerings(self) -> list[Offering]:
        """Generate offerings from Hetzner Cloud plans."""
        offerings: list[Offering] = []

        for plan in HETZNER_PLANS:
            for loc in HETZNER_LOCATIONS:
                # Price adjustment for US/SG locations (about 20% higher)
                price_multiplier = 1.2 if loc["country"] in ("US", "SG") else 1.0
                price_eur = plan["price_eur"] * price_multiplier
                price_usd = price_eur * EUR_TO_USD

                offering = Offering(
                    offering_id=f"hetzner-{plan['id']}-{loc['code']}",
                    offer_name=f"Hetzner Cloud {plan['name']} - {loc['city']}",
                    description=f"Hetzner Cloud {plan['name']} VPS in {loc['city']}, {loc['country']}",
                    product_page_url="https://www.hetzner.com/cloud",
                    currency="USD",
                    monthly_price=round(price_usd, 2),
                    setup_fee=0.0,
                    visibility="public",
                    product_type="compute" if plan["type"] == "shared" else "dedicated",
                    virtualization_type="kvm",
                    billing_interval="monthly",
                    stock_status="in_stock",
                    datacenter_country=loc["country"],
                    datacenter_city=loc["city"],
                    processor_cores=plan["vcpu"],
                    memory_amount=plan["ram"],
                    total_ssd_capacity=plan["ssd"],
                    traffic=loc["traffic_tb"] * 1000,  # Convert TB to GB
                    uplink_speed=1000,  # 1 Gbps
                    unmetered_bandwidth=False,
                    features="IPv4,IPv6,Snapshots,Backups,Firewall",
                    operating_systems="Ubuntu,Debian,Fedora,Rocky,AlmaLinux,CentOS",
                )
                offerings.append(offering)

        return offerings

    def scrape_docs(self) -> list[MarkdownDoc]:
        """Scrape Hetzner documentation pages."""
        docs: list[MarkdownDoc] = []

        # Fetch main cloud page
        try:
            html = self.fetch("https://www.hetzner.com/cloud/")
            doc = html_to_markdown(
                html,
                source_url="https://www.hetzner.com/cloud/",
                provider=self.provider_name,
                topic="Cloud VPS Overview",
            )
            docs.append(doc)
        except Exception as e:
            print(f"Failed to fetch cloud page: {e}")

        # Fetch FAQ/support pages
        faq_urls = [
            ("https://docs.hetzner.com/cloud/", "Cloud Documentation"),
            ("https://www.hetzner.com/cloud#pricing", "Cloud Pricing"),
        ]

        for url, topic in faq_urls:
            try:
                html = self.fetch(url)
                doc = html_to_markdown(
                    html,
                    source_url=url,
                    provider=self.provider_name,
                    topic=topic,
                )
                docs.append(doc)
            except Exception as e:
                print(f"Failed to fetch {url}: {e}")

        return docs


def main() -> None:
    """Run the Hetzner scraper."""
    from pathlib import Path

    output_dir = Path(__file__).parent.parent.parent / "output" / "hetzner"

    with HetznerScraper(output_dir) as scraper:
        csv_path, md_paths = scraper.run()
        print(f"CSV written to: {csv_path}")
        print(f"Markdown files: {len(md_paths)}")
        for path in md_paths:
            size = path.stat().st_size
            print(f"  - {path.name} ({size} bytes)")


if __name__ == "__main__":
    main()
