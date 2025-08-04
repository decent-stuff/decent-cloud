use provider_offering::{
    BillingInterval, Currency, ErrorCorrection, ProductType, StockStatus, VirtualizationType,
    Visibility,
};
use std::str::FromStr;
mod common;
use common::*;

#[test]
fn test_currency_from_str() {
    // Test valid currency parsing
    let valid_cases = vec![
        ("EUR", Currency::EUR),
        ("USD", Currency::USD),
        ("USDT", Currency::USDT),
        ("BTC", Currency::BTC),
        ("ETH", Currency::ETH),
        ("eur", Currency::EUR),
        ("UsD", Currency::USD),
    ];

    test_enum_case_insensitive(valid_cases, Currency::from_str);

    // Test invalid currency
    let invalid_inputs = vec!["INVALID", ""];
    test_enum_invalid_parsing(invalid_inputs, Currency::from_str);
}

#[test]
fn test_currency_display() {
    for (currency, expected) in test_currency_cases() {
        assert_eq!(format!("{}", currency), expected);
    }
}

#[test]
fn test_visibility_from_str() {
    // Test valid visibility parsing
    let valid_cases = vec![
        ("visible", Visibility::Visible),
        ("invisible", Visibility::Invisible),
        ("Visible", Visibility::Visible),
        ("INVISIBLE", Visibility::Invisible),
    ];

    test_enum_case_insensitive(valid_cases, Visibility::from_str);

    // Test invalid visibility
    let invalid_inputs = vec!["INVALID", ""];
    test_enum_invalid_parsing(invalid_inputs, Visibility::from_str);
}

#[test]
fn test_visibility_display() {
    for (visibility, expected) in test_visibility_cases() {
        assert_eq!(format!("{}", visibility), expected);
    }
}

#[test]
fn test_product_type_from_str() {
    // Test valid product type parsing
    let valid_cases = vec![
        ("vps", ProductType::VPS),
        ("dedicated", ProductType::Dedicated),
        ("cloud", ProductType::Cloud),
        ("managed", ProductType::Managed),
        ("VPS", ProductType::VPS),
        ("Cloud", ProductType::Cloud),
    ];

    test_enum_case_insensitive(valid_cases, ProductType::from_str);

    // Test invalid product type
    let invalid_inputs = vec!["INVALID", ""];
    test_enum_invalid_parsing(invalid_inputs, ProductType::from_str);
}

#[test]
fn test_product_type_display() {
    for (product_type, expected) in test_product_type_cases() {
        assert_eq!(format!("{}", product_type), expected);
    }
}

#[test]
fn test_virtualization_type_from_str() {
    // Test valid virtualization type parsing
    let valid_cases = vec![
        ("kvm", VirtualizationType::KVM),
        ("vmware", VirtualizationType::VMware),
        ("xen", VirtualizationType::Xen),
        ("hyper-v", VirtualizationType::HyperV),
        ("hyperv", VirtualizationType::HyperV),
        ("none", VirtualizationType::None),
        ("", VirtualizationType::None),
        ("KVM", VirtualizationType::KVM),
        ("XEN", VirtualizationType::Xen),
    ];

    test_enum_case_insensitive(valid_cases, VirtualizationType::from_str);

    // Test invalid virtualization type
    let invalid_inputs = vec!["INVALID"];
    test_enum_invalid_parsing(invalid_inputs, VirtualizationType::from_str);
}

#[test]
fn test_virtualization_type_display() {
    for (virt_type, expected) in test_virtualization_type_cases() {
        assert_eq!(format!("{}", virt_type), expected);
    }
}

#[test]
fn test_billing_interval_from_str() {
    // Test valid billing interval parsing
    let valid_cases = vec![
        ("hourly", BillingInterval::Hourly),
        ("hour", BillingInterval::Hourly),
        ("daily", BillingInterval::Daily),
        ("day", BillingInterval::Daily),
        ("monthly", BillingInterval::Monthly),
        ("month", BillingInterval::Monthly),
        ("yearly", BillingInterval::Yearly),
        ("year", BillingInterval::Yearly),
        ("Hourly", BillingInterval::Hourly),
        ("MONTHLY", BillingInterval::Monthly),
    ];

    test_enum_case_insensitive(valid_cases, BillingInterval::from_str);

    // Test invalid billing interval
    let invalid_inputs = vec!["INVALID", ""];
    test_enum_invalid_parsing(invalid_inputs, BillingInterval::from_str);
}

#[test]
fn test_billing_interval_display() {
    for (billing_interval, expected) in test_billing_interval_cases() {
        assert_eq!(format!("{}", billing_interval), expected);
    }
}

#[test]
fn test_stock_status_from_str() {
    // Test valid stock status parsing
    let valid_cases = vec![
        ("in stock", StockStatus::InStock),
        ("in-stock", StockStatus::InStock),
        ("out of stock", StockStatus::OutOfStock),
        ("out-of-stock", StockStatus::OutOfStock),
        ("limited", StockStatus::Limited),
        ("In Stock", StockStatus::InStock),
        ("OUT-OF-STOCK", StockStatus::OutOfStock),
        ("Limited", StockStatus::Limited),
    ];

    test_enum_case_insensitive(valid_cases, StockStatus::from_str);

    // Test invalid stock status
    let invalid_inputs = vec!["INVALID", ""];
    test_enum_invalid_parsing(invalid_inputs, StockStatus::from_str);
}

#[test]
fn test_stock_status_display() {
    for (stock_status, expected) in test_stock_status_cases() {
        assert_eq!(format!("{}", stock_status), expected);
    }
}

#[test]
fn test_error_correction_from_str() {
    // Test valid error correction parsing
    let valid_cases = vec![
        ("ecc", ErrorCorrection::ECC),
        ("ecc registered", ErrorCorrection::ECCRegistered),
        ("ecc-registered", ErrorCorrection::ECCRegistered),
        ("ecc-reg", ErrorCorrection::ECCRegistered),
        ("eccreg", ErrorCorrection::ECCRegistered),
        ("non-ecc", ErrorCorrection::NonECC),
        ("nonecc", ErrorCorrection::NonECC),
        ("non ecc", ErrorCorrection::NonECC),
        ("ECC", ErrorCorrection::ECC),
        ("ECC-REGISTERED", ErrorCorrection::ECCRegistered),
        ("NON-ECC", ErrorCorrection::NonECC),
    ];

    test_enum_case_insensitive(valid_cases, ErrorCorrection::from_str);

    // Test invalid error correction
    let invalid_inputs = vec!["INVALID", ""];
    test_enum_invalid_parsing(invalid_inputs, ErrorCorrection::from_str);
}

#[test]
fn test_error_correction_display() {
    for (error_correction, expected) in test_error_correction_cases() {
        assert_eq!(format!("{}", error_correction), expected);
    }
}

#[test]
fn test_currency_roundtrip() {
    for (original, display_str) in test_currency_cases() {
        let display = format!("{}", original);
        assert_eq!(display, display_str);
        let parsed = Currency::from_str(&display).unwrap();
        assert_eq!(
            std::mem::discriminant(&original),
            std::mem::discriminant(&parsed)
        );
    }
}

#[test]
fn test_visibility_roundtrip() {
    for (original, display_str) in test_visibility_cases() {
        let display = format!("{}", original);
        assert_eq!(display, display_str);
        let parsed = Visibility::from_str(&display.to_lowercase()).unwrap();
        assert_eq!(
            std::mem::discriminant(&original),
            std::mem::discriminant(&parsed)
        );
    }
}

#[test]
fn test_product_type_roundtrip() {
    for (original, display_str) in test_product_type_cases() {
        let display = format!("{}", original);
        assert_eq!(display, display_str);
        let parsed = ProductType::from_str(&display.to_lowercase()).unwrap();
        assert_eq!(
            std::mem::discriminant(&original),
            std::mem::discriminant(&parsed)
        );
    }
}

#[test]
fn test_virtualization_type_roundtrip() {
    for (original, display_str) in test_virtualization_type_cases() {
        let display = format!("{}", original);
        assert_eq!(display, display_str);

        // Special case for None which parses from empty string
        let parsed = if matches!(original, VirtualizationType::None) {
            VirtualizationType::from_str("").unwrap()
        } else {
            VirtualizationType::from_str(&display.to_lowercase()).unwrap()
        };
        assert_eq!(
            std::mem::discriminant(&original),
            std::mem::discriminant(&parsed)
        );
    }
}

#[test]
fn test_billing_interval_roundtrip() {
    for (original, display_str) in test_billing_interval_cases() {
        let display = format!("{}", original);
        assert_eq!(display, display_str);
        let parsed = BillingInterval::from_str(&display.to_lowercase()).unwrap();
        assert_eq!(
            std::mem::discriminant(&original),
            std::mem::discriminant(&parsed)
        );
    }
}

#[test]
fn test_stock_status_roundtrip() {
    for (original, display_str) in test_stock_status_cases() {
        let display = format!("{}", original);
        assert_eq!(display, display_str);

        let parsed = match original {
            StockStatus::InStock => StockStatus::from_str("in stock").unwrap(),
            StockStatus::OutOfStock => StockStatus::from_str("out of stock").unwrap(),
            StockStatus::Limited => StockStatus::from_str("limited").unwrap(),
        };
        assert_eq!(
            std::mem::discriminant(&original),
            std::mem::discriminant(&parsed)
        );
    }
}

#[test]
fn test_error_correction_roundtrip() {
    for (original, display_str) in test_error_correction_cases() {
        let display = format!("{}", original);
        assert_eq!(display, display_str);

        let parsed = match original {
            ErrorCorrection::ECC => ErrorCorrection::from_str("ecc").unwrap(),
            ErrorCorrection::ECCRegistered => ErrorCorrection::from_str("ecc registered").unwrap(),
            ErrorCorrection::NonECC => ErrorCorrection::from_str("non-ecc").unwrap(),
        };
        assert_eq!(
            std::mem::discriminant(&original),
            std::mem::discriminant(&parsed)
        );
    }
}
