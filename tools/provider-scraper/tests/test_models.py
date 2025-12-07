"""Tests for the Offering model."""

import pytest
from pydantic import ValidationError

from scraper.models import Offering


def test_offering_minimal_valid() -> None:
    """Test creating an offering with only required fields."""
    offering = Offering(
        offering_id="test-1",
        offer_name="Test Server",
        monthly_price=10.0,
        product_type="compute",
        datacenter_country="US",
        datacenter_city="New York",
    )
    assert offering.offering_id == "test-1"
    assert offering.currency == "USD"  # default
    assert offering.visibility == "public"  # default
    assert offering.unmetered_bandwidth is False  # default


def test_offering_all_fields() -> None:
    """Test creating an offering with all fields populated."""
    offering = Offering(
        offering_id="full-1",
        offer_name="Full Server",
        currency="EUR",
        monthly_price=99.99,
        setup_fee=50.0,
        visibility="public",
        product_type="dedicated",
        billing_interval="monthly",
        stock_status="in_stock",
        datacenter_country="DE",
        datacenter_city="Frankfurt",
        unmetered_bandwidth=True,
        description="A fully loaded server",
        product_page_url="https://example.com/server",
        virtualization_type="kvm",
        processor_brand="AMD",
        processor_amount=2,
        processor_cores=64,
        processor_speed=3.5,
        processor_name="EPYC 7763",
        memory_error_correction="ECC",
        memory_type="DDR5",
        memory_amount=256,
        hdd_amount=0,
        total_hdd_capacity=0,
        ssd_amount=2,
        total_ssd_capacity=2000,
        uplink_speed=10000,
        traffic=None,  # unlimited
        datacenter_latitude=50.1109,
        datacenter_longitude=8.6821,
        control_panel="None",
        gpu_name="NVIDIA A100",
        gpu_count=4,
        gpu_memory_gb=320,
        min_contract_hours=720,
        max_contract_hours=8760,
        payment_methods="BTC,ETH,USDC",
        features="DDoS Protection,IPv6",
        operating_systems="Ubuntu,Debian,Rocky",
    )
    assert offering.processor_cores == 64
    assert offering.gpu_count == 4


def test_offering_negative_price_rejected() -> None:
    """Test that negative prices are rejected."""
    with pytest.raises(ValidationError):
        Offering(
            offering_id="bad-1",
            offer_name="Bad Server",
            monthly_price=-10.0,
            product_type="compute",
            datacenter_country="US",
            datacenter_city="NYC",
        )


def test_offering_missing_required_field() -> None:
    """Test that missing required fields raise validation errors."""
    with pytest.raises(ValidationError):
        Offering(
            offering_id="incomplete",
            offer_name="Incomplete",
            # missing: monthly_price, product_type, datacenter_country, datacenter_city
        )  # type: ignore[call-arg]
