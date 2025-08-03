use np_offering::OfferingError;
use std::io;

#[test]
fn test_error_creation() {
    // Test ParseError creation
    let parse_error = OfferingError::ParseError("Test parse error".to_string());
    assert!(matches!(parse_error, OfferingError::ParseError(_)));

    // Test IoError creation
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let offering_io_error = OfferingError::IoError(io_error);
    assert!(matches!(offering_io_error, OfferingError::IoError(_)));

    // Test CsvError creation
    let csv_error = csv::Error::from(io::Error::new(io::ErrorKind::InvalidData, "CSV error"));
    let offering_csv_error = OfferingError::CsvError(csv_error);
    assert!(matches!(offering_csv_error, OfferingError::CsvError(_)));

    // Test SerdeJsonError creation
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let offering_json_error = OfferingError::SerdeJsonError(json_error);
    assert!(matches!(
        offering_json_error,
        OfferingError::SerdeJsonError(_)
    ));

    // Test InvalidPubkeyLength creation
    let pubkey_error = OfferingError::InvalidPubkeyLength(16);
    assert!(matches!(
        pubkey_error,
        OfferingError::InvalidPubkeyLength(_)
    ));

    // Test OfferingNotFound creation
    let offering_not_found =
        OfferingError::OfferingNotFound("provider1".to_string(), "key1".to_string());
    assert!(matches!(
        offering_not_found,
        OfferingError::OfferingNotFound(_, _)
    ));

    // Test ProviderNotFound creation
    let provider_not_found = OfferingError::ProviderNotFound("provider1".to_string());
    assert!(matches!(
        provider_not_found,
        OfferingError::ProviderNotFound(_)
    ));
}

#[test]
fn test_error_display() {
    // Test ParseError display
    let parse_error = OfferingError::ParseError("Test parse error".to_string());
    assert_eq!(format!("{}", parse_error), "Parse error: Test parse error");

    // Test IoError display
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let offering_io_error = OfferingError::IoError(io_error);
    let display_str = format!("{}", offering_io_error);
    assert!(display_str.contains("IO error"));
    assert!(display_str.contains("File not found"));

    // Test CsvError display
    let csv_error = csv::Error::from(io::Error::new(io::ErrorKind::InvalidData, "CSV error"));
    let offering_csv_error = OfferingError::CsvError(csv_error);
    let display_str = format!("{}", offering_csv_error);
    assert!(display_str.contains("CSV error"));

    // Test SerdeJsonError display
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let offering_json_error = OfferingError::SerdeJsonError(json_error);
    let display_str = format!("{}", offering_json_error);
    assert!(display_str.contains("Serde JSON error"));

    // Test InvalidPubkeyLength display
    let pubkey_error = OfferingError::InvalidPubkeyLength(16);
    assert_eq!(
        format!("{}", pubkey_error),
        "Invalid provider pubkey: expected 32 bytes, got 16"
    );

    // Test OfferingNotFound display
    let offering_not_found =
        OfferingError::OfferingNotFound("provider1".to_string(), "key1".to_string());
    assert_eq!(
        format!("{}", offering_not_found),
        "Offering not found: provider=provider1, key=key1"
    );

    // Test ProviderNotFound display
    let provider_not_found = OfferingError::ProviderNotFound("provider1".to_string());
    assert_eq!(
        format!("{}", provider_not_found),
        "Provider not found: provider1"
    );
}

#[test]
fn test_error_from_io() {
    // Test From trait for io::Error
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let offering_error: OfferingError = io_error.into();
    assert!(matches!(offering_error, OfferingError::IoError(_)));
}

#[test]
fn test_error_from_csv() {
    // Test From trait for csv::Error
    let csv_error = csv::Error::from(io::Error::new(io::ErrorKind::InvalidData, "CSV error"));
    let offering_error: OfferingError = csv_error.into();
    assert!(matches!(offering_error, OfferingError::CsvError(_)));
}

#[test]
fn test_error_from_json() {
    // Test From trait for serde_json::Error
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let offering_error: OfferingError = json_error.into();
    assert!(matches!(offering_error, OfferingError::SerdeJsonError(_)));
}

#[test]
fn test_error_debug_format() {
    // Test Debug implementation for all error types
    let parse_error = OfferingError::ParseError("Test".to_string());
    let debug_str = format!("{:?}", parse_error);
    assert!(debug_str.contains("ParseError"));

    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let offering_io_error = OfferingError::IoError(io_error);
    let debug_str = format!("{:?}", offering_io_error);
    assert!(debug_str.contains("IoError"));

    let csv_error = csv::Error::from(io::Error::new(io::ErrorKind::InvalidData, "CSV error"));
    let offering_csv_error = OfferingError::CsvError(csv_error);
    let debug_str = format!("{:?}", offering_csv_error);
    assert!(debug_str.contains("CsvError"));

    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let offering_json_error = OfferingError::SerdeJsonError(json_error);
    let debug_str = format!("{:?}", offering_json_error);
    assert!(debug_str.contains("SerdeJsonError"));

    let pubkey_error = OfferingError::InvalidPubkeyLength(16);
    let debug_str = format!("{:?}", pubkey_error);
    assert!(debug_str.contains("InvalidPubkeyLength"));

    let offering_not_found =
        OfferingError::OfferingNotFound("provider1".to_string(), "key1".to_string());
    let debug_str = format!("{:?}", offering_not_found);
    assert!(debug_str.contains("OfferingNotFound"));

    let provider_not_found = OfferingError::ProviderNotFound("provider1".to_string());
    let debug_str = format!("{:?}", provider_not_found);
    assert!(debug_str.contains("ProviderNotFound"));
}

#[test]
fn test_error_send_sync() {
    // Test that errors are Send and Sync (required for threading)
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<OfferingError>();
}
