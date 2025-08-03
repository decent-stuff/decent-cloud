//! Common test utilities and shared test data for the np-offering crate

#![allow(dead_code)]

use ed25519_dalek::{SigningKey, VerifyingKey, SECRET_KEY_LENGTH};
use np_offering::{
    BillingInterval, Currency, ErrorCorrection, ProductType, ProviderPubkey, StockStatus,
    VirtualizationType, Visibility,
};

/// Sample CSV data with multiple offerings for testing
pub const SAMPLE_CSV: &str = r#"Offer Name,Description,Unique Internal identifier,Product page URL,Currency,Monthly price,Setup fee,Visibility,Product Type,Virtualization type,Billing interval,Stock,Processor Brand,Processor Amount,Processor Cores,Processor Speed,Processor Name,Memory Error Correction,Memory Type,Memory Amount,Hard Disk Drive Amount,Total Hard Disk Drive Capacity,Solid State Disk Amount,Total Solid State Disk Capacity,Unmetered,Uplink speed,Traffic,Datacenter Country,Datacenter City,Datacenter Coordinates,Features,Operating Systems,Control Panel,GPU Name,Payment Methods
Intel Dual Core Dedicated Server,Here goes a product description.,DC2993,https://test.com/DC2993/,EUR,99.99,99.99,Visible,VPS,KVM,Monthly,In stock,Intel,1,2,2.6 GHz,Intel® Xeon® Processor E5-1620 v4,non-ECC,DDR4,8192 MB,0,0,2,160 GB,Unmetered inbound,1000 mbit,10240,NL,"Rotterdam, Netherlands","51.9229,4.46317","KVM over IP, Managed support, Native IPv6, Instant setup","Debian, CentOs, VMWare",cPanel,,"Bitcoin, Credit card, PayPal, Wire Transfer"
Intel Quad Core VPS,Another product description.,QC1494,https://test.com/QC1494/,USD,149.99,0.0,Visible,VPS,KVM,Monthly,In stock,Intel,1,4,2200 MHz,Intel® Xeon® Processor E3-1505L v6,ECC,DDR4,16 GB,0,0,1,240 GB,Unmetered inbound,1000 mbit,5120,US,"New York, NY","40.7128,-74.0060","KVM over IP, SSD Storage, IPv6","Ubuntu, CentOS, Debian",cPanel,NVIDIA GTX 1080,"Credit card, PayPal"
Budget Server,Cheap option for startups.,BS001,https://test.com/BS001/,USD,29.99,0.0,Visible,VPS,None,Monthly,In stock,AMD,1,1,2.0 GHz,AMD Opteron,non-ECC,DDR3,2 GB,1,500 GB,0,0,Standard,100 mbit,1024,US,"Dallas, TX","32.7767,-96.7970","Basic support","Ubuntu, CentOS",,,"PayPal""#;

/// Single offering CSV for simpler tests
pub const SINGLE_OFFERING_CSV: &str = r#"Offer Name,Description,Unique Internal identifier,Product page URL,Currency,Monthly price,Setup fee,Visibility,Product Type,Virtualization type,Billing interval,Stock,Processor Brand,Processor Amount,Processor Cores,Processor Speed,Processor Name,Memory Error Correction,Memory Type,Memory Amount,Hard Disk Drive Amount,Total Hard Disk Drive Capacity,Solid State Disk Amount,Total Solid State Disk Capacity,Unmetered,Uplink speed,Traffic,Datacenter Country,Datacenter City,Datacenter Coordinates,Features,Operating Systems,Control Panel,GPU Name,Payment Methods
Intel Dual Core Dedicated Server,Here goes a product description.,DC2993,https://test.com/DC2993/,EUR,99.99,99.99,Visible,VPS,KVM,Monthly,In stock,Intel,1,2,2.6 GHz,Intel® Xeon® Processor E5-1620 v4,non-ECC,DDR4,8192 MB,0,0,2,160 GB,Unmetered inbound,1000 mbit,10240,NL,"Rotterdam, Netherlands","51.9229,4.46317","KVM over IP, Managed support, Native IPv6, Instant setup","Debian, CentOs, VMWare",cPanel,,"Bitcoin, Credit card, PayPal, Wire Transfer""#;

/// Create a test provider pubkey with predictable values
pub fn test_provider_pubkey(id: u8) -> ProviderPubkey {
    let seed: [u8; SECRET_KEY_LENGTH] = [id; 32];
    let sk = SigningKey::from_bytes(&seed);
    let vk: VerifyingKey = sk.verifying_key();
    ProviderPubkey::new(vk.to_bytes())
}

/// Create a test provider pubkey with default value (all 1s)
pub fn default_test_provider_pubkey() -> ProviderPubkey {
    test_provider_pubkey(1)
}

/// Test data for enum roundtrip testing
pub fn test_currency_cases() -> Vec<(Currency, &'static str)> {
    vec![
        (Currency::EUR, "EUR"),
        (Currency::USD, "USD"),
        (Currency::USDT, "USDT"),
        (Currency::BTC, "BTC"),
        (Currency::ETH, "ETH"),
    ]
}

pub fn test_visibility_cases() -> Vec<(Visibility, &'static str)> {
    vec![
        (Visibility::Visible, "Visible"),
        (Visibility::Invisible, "Invisible"),
    ]
}

pub fn test_product_type_cases() -> Vec<(ProductType, &'static str)> {
    vec![
        (ProductType::VPS, "VPS"),
        (ProductType::Dedicated, "Dedicated"),
        (ProductType::Cloud, "Cloud"),
        (ProductType::Managed, "Managed"),
    ]
}

pub fn test_virtualization_type_cases() -> Vec<(VirtualizationType, &'static str)> {
    vec![
        (VirtualizationType::KVM, "KVM"),
        (VirtualizationType::VMware, "VMware"),
        (VirtualizationType::Xen, "Xen"),
        (VirtualizationType::HyperV, "Hyper-V"),
        (VirtualizationType::None, "None"),
    ]
}

pub fn test_billing_interval_cases() -> Vec<(BillingInterval, &'static str)> {
    vec![
        (BillingInterval::Hourly, "Hourly"),
        (BillingInterval::Daily, "Daily"),
        (BillingInterval::Monthly, "Monthly"),
        (BillingInterval::Yearly, "Yearly"),
    ]
}

pub fn test_stock_status_cases() -> Vec<(StockStatus, &'static str)> {
    vec![
        (StockStatus::InStock, "In stock"),
        (StockStatus::OutOfStock, "Out of stock"),
        (StockStatus::Limited, "Limited"),
    ]
}

pub fn test_error_correction_cases() -> Vec<(ErrorCorrection, &'static str)> {
    vec![
        (ErrorCorrection::ECC, "ECC"),
        (ErrorCorrection::ECCRegistered, "ECC Registered"),
        (ErrorCorrection::NonECC, "non-ECC"),
    ]
}

/// Test case insensitive parsing for enums
pub fn test_enum_case_insensitive<T, P>(test_cases: Vec<(&str, T)>, parse_func: P)
where
    T: std::fmt::Debug,
    P: Fn(&str) -> Result<T, ()>,
{
    for (input, expected) in test_cases {
        let parsed = parse_func(input).unwrap_or_else(|_| {
            panic!("Failed to parse enum value: {}", input);
        });
        assert_eq!(
            std::mem::discriminant(&parsed),
            std::mem::discriminant(&expected)
        );
    }
}

/// Test invalid enum parsing
pub fn test_enum_invalid_parsing<T, P>(invalid_inputs: Vec<&str>, parse_func: P)
where
    P: Fn(&str) -> Result<T, ()>,
{
    for input in invalid_inputs {
        assert!(
            parse_func(input).is_err(),
            "Expected error for input: {}",
            input
        );
    }
}
