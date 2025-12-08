"""OVH Cloud scraper."""

from typing import TypedDict

from scraper.base import BaseScraper
from scraper.models import Offering


class OvhPlan(TypedDict):
    """Type for OVH plan data."""

    id: str
    name: str
    vcpu: int
    ram: int
    ssd: int
    price_usd: float
    uplink: int  # Mbps
    product_type: str


class OvhLocation(TypedDict):
    """Type for OVH location data."""

    code: str
    city: str
    country: str


# OVH VPS plans (2025 pricing)
# Data from https://www.ovhcloud.com/en/vps/ as of Dec 2025
OVH_VPS_PLANS: list[OvhPlan] = [
    {"id": "vps-1", "name": "VPS-1", "vcpu": 4, "ram": 8, "ssd": 75, "price_usd": 4.20, "uplink": 400, "product_type": "compute"},
    {"id": "vps-2", "name": "VPS-2", "vcpu": 6, "ram": 12, "ssd": 100, "price_usd": 6.75, "uplink": 1000, "product_type": "compute"},
    {"id": "vps-3", "name": "VPS-3", "vcpu": 8, "ram": 24, "ssd": 200, "price_usd": 12.75, "uplink": 1500, "product_type": "compute"},
    {"id": "vps-4", "name": "VPS-4", "vcpu": 12, "ram": 48, "ssd": 300, "price_usd": 22.08, "uplink": 2000, "product_type": "compute"},
    {"id": "vps-5", "name": "VPS-5", "vcpu": 16, "ram": 64, "ssd": 350, "price_usd": 34.34, "uplink": 2500, "product_type": "compute"},
    {"id": "vps-6", "name": "VPS-6", "vcpu": 24, "ram": 96, "ssd": 400, "price_usd": 45.39, "uplink": 3000, "product_type": "compute"},
]

# OVH Dedicated server base plans (Rise series - entry level)
# Data from https://www.ovhcloud.com/en/bare-metal/ as of Dec 2025
OVH_DEDICATED_PLANS: list[OvhPlan] = [
    {"id": "rise-1", "name": "Rise-1", "vcpu": 4, "ram": 32, "ssd": 500, "price_usd": 53.10, "uplink": 1000, "product_type": "dedicated"},
    {"id": "rise-2", "name": "Rise-2", "vcpu": 6, "ram": 64, "ssd": 1000, "price_usd": 65.00, "uplink": 1000, "product_type": "dedicated"},
    {"id": "rise-3", "name": "Rise-3", "vcpu": 8, "ram": 128, "ssd": 2000, "price_usd": 85.00, "uplink": 3000, "product_type": "dedicated"},
    {"id": "advance-1", "name": "Advance-1", "vcpu": 8, "ram": 64, "ssd": 1000, "price_usd": 81.00, "uplink": 3000, "product_type": "dedicated"},
    {"id": "advance-2", "name": "Advance-2", "vcpu": 16, "ram": 128, "ssd": 2000, "price_usd": 120.00, "uplink": 3000, "product_type": "dedicated"},
    {"id": "advance-3", "name": "Advance-3", "vcpu": 32, "ram": 256, "ssd": 4000, "price_usd": 200.00, "uplink": 5000, "product_type": "dedicated"},
]

# OVH datacenter locations (major ones)
OVH_LOCATIONS: list[OvhLocation] = [
    {"code": "gra", "city": "Gravelines", "country": "FR"},
    {"code": "sbg", "city": "Strasbourg", "country": "FR"},
    {"code": "rbx", "city": "Roubaix", "country": "FR"},
    {"code": "lon", "city": "London", "country": "GB"},
    {"code": "fra", "city": "Frankfurt", "country": "DE"},
    {"code": "waw", "city": "Warsaw", "country": "PL"},
    {"code": "bhs", "city": "Beauharnois", "country": "CA"},
    {"code": "vint", "city": "Vint Hill", "country": "US"},
    {"code": "hil", "city": "Hillsboro", "country": "US"},
    {"code": "sgp", "city": "Singapore", "country": "SG"},
    {"code": "syd", "city": "Sydney", "country": "AU"},
]


class OvhScraper(BaseScraper):
    """Scraper for OVH Cloud offerings."""

    provider_name = "OVH"
    provider_website = "https://www.ovhcloud.com"

    async def scrape_offerings(self) -> list[Offering]:
        """Generate offerings from OVH plans."""
        offerings: list[Offering] = []

        all_plans = OVH_VPS_PLANS + OVH_DEDICATED_PLANS

        for plan in all_plans:
            for loc in OVH_LOCATIONS:
                offering = Offering(
                    offering_id=f"ovh-{plan['id']}-{loc['code']}",
                    offer_name=f"OVH {plan['name']} - {loc['city']}",
                    description=f"OVH {plan['name']} in {loc['city']}, {loc['country']}",
                    product_page_url="https://www.ovhcloud.com/en/vps/" if plan["product_type"] == "compute" else "https://www.ovhcloud.com/en/bare-metal/",
                    currency="USD",
                    monthly_price=plan["price_usd"],
                    setup_fee=0.0,
                    visibility="public",
                    product_type=plan["product_type"],
                    virtualization_type="kvm" if plan["product_type"] == "compute" else None,
                    billing_interval="monthly",
                    stock_status="in_stock",
                    datacenter_country=loc["country"],
                    datacenter_city=loc["city"],
                    processor_cores=plan["vcpu"],
                    memory_amount=plan["ram"],
                    total_ssd_capacity=plan["ssd"],
                    uplink_speed=plan["uplink"],
                    unmetered_bandwidth=True,  # OVH has unlimited traffic
                    features="DDoS Protection,IPv4,IPv6,Daily Backup",
                    operating_systems="Ubuntu,Debian,CentOS,Rocky,AlmaLinux,Fedora,Windows",
                )
                offerings.append(offering)

        return offerings


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
