"""Tests for the CSV writer."""

from pathlib import Path
from tempfile import TemporaryDirectory

from scraper.csv_writer import (
    CSV_HEADERS,
    offerings_to_csv_string,
    write_offerings_csv,
)
from scraper.models import Offering


def test_csv_headers_count() -> None:
    """Test that we have exactly 40 headers matching the API schema."""
    assert len(CSV_HEADERS) == 40


def test_csv_headers_required_fields() -> None:
    """Test that required fields are in headers."""
    required = [
        "offering_id",
        "offer_name",
        "currency",
        "monthly_price",
        "product_type",
        "datacenter_country",
        "datacenter_city",
    ]
    for field in required:
        assert field in CSV_HEADERS


def test_offerings_to_csv_string_minimal() -> None:
    """Test CSV output with minimal offering."""
    offering = Offering(
        offering_id="test-1",
        offer_name="Test Server",
        monthly_price=10.0,
        product_type="compute",
        datacenter_country="US",
        datacenter_city="New York",
    )
    csv_str = offerings_to_csv_string([offering])
    lines = csv_str.strip().replace("\r", "").split("\n")

    # First line is header
    assert lines[0] == ",".join(CSV_HEADERS)

    # Second line is data
    data_line = lines[1]
    assert "test-1" in data_line
    assert "Test Server" in data_line
    assert "10" in data_line  # price
    assert "US" in data_line
    assert "New York" in data_line


def test_offerings_to_csv_string_boolean_format() -> None:
    """Test that booleans are formatted as 'true'/'false'."""
    offering = Offering(
        offering_id="bool-test",
        offer_name="Bool Test",
        monthly_price=5.0,
        product_type="compute",
        datacenter_country="US",
        datacenter_city="NYC",
        unmetered_bandwidth=True,
    )
    csv_str = offerings_to_csv_string([offering])
    assert "true" in csv_str  # unmetered_bandwidth


def test_write_offerings_csv_creates_file() -> None:
    """Test that write_offerings_csv creates a file."""
    offering = Offering(
        offering_id="file-test",
        offer_name="File Test",
        monthly_price=20.0,
        product_type="dedicated",
        datacenter_country="DE",
        datacenter_city="Frankfurt",
    )
    with TemporaryDirectory() as tmpdir:
        path = Path(tmpdir) / "output" / "test.csv"
        write_offerings_csv([offering], path)

        assert path.exists()
        content = path.read_text()
        assert "file-test" in content
        assert "Frankfurt" in content


def test_csv_float_formatting() -> None:
    """Test that floats are formatted cleanly without trailing zeros."""
    offering = Offering(
        offering_id="float-test",
        offer_name="Float Test",
        monthly_price=10.00,  # should become "10"
        setup_fee=5.50,  # should stay "5.5"
        product_type="compute",
        datacenter_country="US",
        datacenter_city="NYC",
        processor_speed=3.50,  # should become "3.5"
    )
    csv_str = offerings_to_csv_string([offering])
    # Check that we don't have unnecessary decimal places
    assert ",10," in csv_str or csv_str.endswith(",10")  # monthly_price as "10"
    assert "5.5" in csv_str  # setup_fee
    assert "3.5" in csv_str  # processor_speed
