"""CSV writer matching the API import schema exactly."""

import csv
from io import StringIO
from pathlib import Path

from scraper.models import Offering

# CSV header order must match api/src/openapi/offerings.rs exactly
CSV_HEADERS = [
    "offering_id",
    "offer_name",
    "description",
    "product_page_url",
    "currency",
    "monthly_price",
    "setup_fee",
    "visibility",
    "product_type",
    "virtualization_type",
    "billing_interval",
    "stock_status",
    "processor_brand",
    "processor_amount",
    "processor_cores",
    "processor_speed",
    "processor_name",
    "memory_error_correction",
    "memory_type",
    "memory_amount",
    "hdd_amount",
    "total_hdd_capacity",
    "ssd_amount",
    "total_ssd_capacity",
    "unmetered_bandwidth",
    "uplink_speed",
    "traffic",
    "datacenter_country",
    "datacenter_city",
    "datacenter_latitude",
    "datacenter_longitude",
    "control_panel",
    "gpu_name",
    "gpu_count",
    "gpu_memory_gb",
    "min_contract_hours",
    "max_contract_hours",
    "payment_methods",
    "features",
    "operating_systems",
]


def _format_value(value: object) -> str:
    """Format a value for CSV output."""
    if value is None:
        return ""
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, float):
        # Avoid scientific notation, remove trailing zeros
        int_val = int(value)
        if value == float(int_val):
            return str(int_val)
        return f"{value:.2f}".rstrip("0").rstrip(".")
    return str(value)


def offering_to_row(offering: Offering) -> list[str]:
    """Convert an Offering to a CSV row in the correct column order."""
    data = offering.model_dump()
    return [_format_value(data.get(header)) for header in CSV_HEADERS]


def write_offerings_csv(offerings: list[Offering], path: Path) -> None:
    """Write offerings to a CSV file matching the API import schema."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(CSV_HEADERS)
        for offering in offerings:
            writer.writerow(offering_to_row(offering))


def offerings_to_csv_string(offerings: list[Offering]) -> str:
    """Convert offerings to a CSV string for testing."""
    output = StringIO()
    writer = csv.writer(output)
    writer.writerow(CSV_HEADERS)
    for offering in offerings:
        writer.writerow(offering_to_row(offering))
    return output.getvalue()
