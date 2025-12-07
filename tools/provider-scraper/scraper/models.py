"""Offering model matching the API CSV import schema."""

from pydantic import BaseModel, Field


class Offering(BaseModel):
    """Provider offering matching the 41-field CSV schema for import_offerings_csv."""

    # Required fields
    offering_id: str = Field(description="Unique ID (provider SKU or generated)")
    offer_name: str = Field(description="Product name")
    currency: str = Field(default="USD", description="Currency code")
    monthly_price: float = Field(ge=0, description="Price per month")
    setup_fee: float = Field(default=0.0, ge=0, description="One-time setup fee")
    visibility: str = Field(default="public", description="public or private")
    product_type: str = Field(description="compute, dedicated, or gpu")
    billing_interval: str = Field(default="monthly", description="monthly or hourly")
    stock_status: str = Field(default="in_stock", description="in_stock or out_of_stock")
    datacenter_country: str = Field(description="ISO 3166-1 alpha-2 code")
    datacenter_city: str = Field(description="City name")
    unmetered_bandwidth: bool = Field(default=False, description="True if unlimited bandwidth")

    # Optional fields
    description: str | None = Field(default=None)
    product_page_url: str | None = Field(default=None)
    virtualization_type: str | None = Field(default=None, description="kvm, lxc, etc.")
    processor_brand: str | None = Field(default=None, description="Intel, AMD, etc.")
    processor_amount: int | None = Field(default=None, ge=0, description="Number of CPUs")
    processor_cores: int | None = Field(default=None, ge=0, description="Total cores")
    processor_speed: float | None = Field(default=None, ge=0, description="GHz")
    processor_name: str | None = Field(default=None, description="CPU model name")
    memory_error_correction: str | None = Field(default=None, description="ECC status")
    memory_type: str | None = Field(default=None, description="DDR4, DDR5, etc.")
    memory_amount: int | None = Field(default=None, ge=0, description="RAM in GB")
    hdd_amount: int | None = Field(default=None, ge=0, description="Number of HDDs")
    total_hdd_capacity: int | None = Field(default=None, ge=0, description="Total HDD GB")
    ssd_amount: int | None = Field(default=None, ge=0, description="Number of SSDs")
    total_ssd_capacity: int | None = Field(default=None, ge=0, description="Total SSD GB")
    uplink_speed: int | None = Field(default=None, ge=0, description="Mbps")
    traffic: int | None = Field(default=None, ge=0, description="GB per month, None if unlimited")
    datacenter_latitude: float | None = Field(default=None)
    datacenter_longitude: float | None = Field(default=None)
    control_panel: str | None = Field(default=None)
    gpu_name: str | None = Field(default=None)
    gpu_count: int | None = Field(default=None, ge=0)
    gpu_memory_gb: int | None = Field(default=None, ge=0)
    min_contract_hours: int | None = Field(default=None, ge=0)
    max_contract_hours: int | None = Field(default=None, ge=0)
    payment_methods: str | None = Field(default=None, description="Comma-separated")
    features: str | None = Field(default=None, description="Comma-separated")
    operating_systems: str | None = Field(default=None, description="Comma-separated")
