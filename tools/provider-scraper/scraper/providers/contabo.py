"""Contabo scraper."""

from typing import TypedDict

from scraper.base import BaseScraper
from scraper.markdown import MarkdownDoc, html_to_markdown
from scraper.models import Offering


class ContaboPlan(TypedDict):
    """Type for Contabo plan data."""

    id: str
    name: str
    vcpu: int
    ram: int
    nvme: int
    ssd: int
    price_eur: float
    uplink: int  # Mbit/s
    product_type: str  # "compute" or "dedicated"


class ContaboLocation(TypedDict):
    """Type for Contabo location data."""

    code: str
    city: str
    country: str


# Contabo VPS plans (Cloud VPS - shared)
# Data from https://contabo.com/en/vps/ as of Dec 2025
CONTABO_VPS_PLANS: list[ContaboPlan] = [
    {"id": "vps-10", "name": "Cloud VPS 10", "vcpu": 4, "ram": 8, "nvme": 75, "ssd": 150, "price_eur": 4.50, "uplink": 200, "product_type": "compute"},
    {"id": "vps-20", "name": "Cloud VPS 20", "vcpu": 6, "ram": 12, "nvme": 100, "ssd": 200, "price_eur": 7.00, "uplink": 300, "product_type": "compute"},
    {"id": "vps-30", "name": "Cloud VPS 30", "vcpu": 8, "ram": 24, "nvme": 200, "ssd": 400, "price_eur": 14.00, "uplink": 600, "product_type": "compute"},
    {"id": "vps-40", "name": "Cloud VPS 40", "vcpu": 12, "ram": 48, "nvme": 250, "ssd": 500, "price_eur": 25.00, "uplink": 800, "product_type": "compute"},
    {"id": "vps-50", "name": "Cloud VPS 50", "vcpu": 16, "ram": 64, "nvme": 300, "ssd": 600, "price_eur": 37.00, "uplink": 1000, "product_type": "compute"},
    {"id": "vps-60", "name": "Cloud VPS 60", "vcpu": 18, "ram": 96, "nvme": 350, "ssd": 700, "price_eur": 49.00, "uplink": 1000, "product_type": "compute"},
]

# Contabo VDS plans (dedicated cores)
# Data from https://contabo.com/en/vds/ as of Dec 2025
CONTABO_VDS_PLANS: list[ContaboPlan] = [
    {"id": "vds-s", "name": "Cloud VDS S", "vcpu": 3, "ram": 24, "nvme": 180, "ssd": 0, "price_eur": 34.40, "uplink": 250, "product_type": "dedicated"},
    {"id": "vds-m", "name": "Cloud VDS M", "vcpu": 4, "ram": 32, "nvme": 240, "ssd": 0, "price_eur": 44.80, "uplink": 500, "product_type": "dedicated"},
    {"id": "vds-l", "name": "Cloud VDS L", "vcpu": 6, "ram": 48, "nvme": 360, "ssd": 0, "price_eur": 64.00, "uplink": 750, "product_type": "dedicated"},
    {"id": "vds-xl", "name": "Cloud VDS XL", "vcpu": 8, "ram": 64, "nvme": 480, "ssd": 0, "price_eur": 82.40, "uplink": 1000, "product_type": "dedicated"},
    {"id": "vds-xxl", "name": "Cloud VDS XXL", "vcpu": 12, "ram": 96, "nvme": 720, "ssd": 0, "price_eur": 119.00, "uplink": 1000, "product_type": "dedicated"},
]

# Contabo datacenter locations
CONTABO_LOCATIONS: list[ContaboLocation] = [
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

# EUR to USD conversion
EUR_TO_USD = 1.10


class ContaboScraper(BaseScraper):
    """Scraper for Contabo offerings."""

    provider_name = "Contabo"
    provider_website = "https://contabo.com"

    def scrape_offerings(self) -> list[Offering]:
        """Generate offerings from Contabo plans."""
        offerings: list[Offering] = []

        all_plans = CONTABO_VPS_PLANS + CONTABO_VDS_PLANS

        for plan in all_plans:
            for loc in CONTABO_LOCATIONS:
                price_usd = plan["price_eur"] * EUR_TO_USD

                # Use NVMe if available, otherwise SSD
                storage = plan["nvme"] if plan["nvme"] > 0 else plan["ssd"]

                offering = Offering(
                    offering_id=f"contabo-{plan['id']}-{loc['code']}",
                    offer_name=f"Contabo {plan['name']} - {loc['city']}",
                    description=f"Contabo {plan['name']} in {loc['city']}, {loc['country']}",
                    product_page_url="https://contabo.com/en/vps/" if "vps" in plan["id"] else "https://contabo.com/en/vds/",
                    currency="USD",
                    monthly_price=round(price_usd, 2),
                    setup_fee=0.0,
                    visibility="public",
                    product_type=plan["product_type"],
                    virtualization_type="kvm",
                    billing_interval="monthly",
                    stock_status="in_stock",
                    datacenter_country=loc["country"],
                    datacenter_city=loc["city"],
                    processor_brand="AMD" if plan["product_type"] == "dedicated" else None,
                    processor_name="EPYC 7282" if plan["product_type"] == "dedicated" else None,
                    processor_cores=plan["vcpu"],
                    memory_amount=plan["ram"],
                    total_ssd_capacity=storage,
                    uplink_speed=plan["uplink"],
                    unmetered_bandwidth=True,  # Contabo has unlimited traffic
                    features="DDoS Protection,IPv4,IPv6,Snapshots",
                    operating_systems="Ubuntu,Debian,CentOS,Rocky,AlmaLinux,Windows",
                )
                offerings.append(offering)

        return offerings

    def scrape_docs(self) -> list[MarkdownDoc]:
        """Scrape Contabo documentation pages."""
        docs: list[MarkdownDoc] = []

        doc_urls = [
            ("https://contabo.com/en/vps/", "VPS Overview"),
            ("https://contabo.com/en/vds/", "VDS Overview"),
            ("https://contabo.com/en/about-us/", "About Contabo"),
        ]

        for url, topic in doc_urls:
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
    """Run the Contabo scraper."""
    from pathlib import Path

    output_dir = Path(__file__).parent.parent.parent / "output" / "contabo"

    with ContaboScraper(output_dir) as scraper:
        csv_path, md_paths = scraper.run()
        print(f"CSV written to: {csv_path}")
        print(f"Markdown files: {len(md_paths)}")
        for path in md_paths:
            size = path.stat().st_size
            print(f"  - {path.name} ({size} bytes)")


if __name__ == "__main__":
    main()
