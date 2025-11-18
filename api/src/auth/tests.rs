use super::*;
use dcc_common::DccIdentity;
use ts_rs::TS;

fn create_test_identity() -> (DccIdentity, Vec<u8>) {
    let seed = [42u8; 32];
    let identity = DccIdentity::new_from_seed(&seed).unwrap();
    let pubkey = identity.to_bytes_verifying();
    (identity, pubkey)
}

#[test]
fn test_verify_valid_signature() {
    let (identity, pubkey) = create_test_identity();
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();
    let nonce = uuid::Uuid::new_v4();
    let method = "POST";
    let path = "/api/v1/users/abc123/profile";
    let body = b"{\"display_name\":\"Test User\"}";

    // Construct message: timestamp + nonce + method + path + body
    let timestamp_str = timestamp.to_string();
    let nonce_str = nonce.to_string();
    let mut message = timestamp_str.as_bytes().to_vec();
    message.extend_from_slice(nonce_str.as_bytes());
    message.extend_from_slice(method.as_bytes());
    message.extend_from_slice(path.as_bytes());
    message.extend_from_slice(body);

    // Sign message
    let signature = identity.sign(&message).unwrap();

    // Verify
    let result = verify_request_signature(
        &hex::encode(&pubkey),
        &hex::encode(signature.to_bytes()),
        &timestamp_str,
        &nonce_str,
        method,
        path,
        body,
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), pubkey);
}

#[test]
fn test_verify_invalid_signature() {
    let (identity, pubkey) = create_test_identity();
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();
    let nonce = uuid::Uuid::new_v4();
    let method = "POST";
    let path = "/api/v1/users/abc123/profile";
    let body = b"{\"display_name\":\"Test User\"}";

    // Construct message: timestamp + nonce + method + path + body
    let timestamp_str = timestamp.to_string();
    let nonce_str = nonce.to_string();
    let mut message = timestamp_str.as_bytes().to_vec();
    message.extend_from_slice(nonce_str.as_bytes());
    message.extend_from_slice(method.as_bytes());
    message.extend_from_slice(path.as_bytes());
    message.extend_from_slice(body);

    // Sign message
    let signature = identity.sign(&message).unwrap();

    // Tamper with body
    let tampered_body = b"{\"display_name\":\"Hacker\"}";

    // Verify should fail
    let result = verify_request_signature(
        &hex::encode(&pubkey),
        &hex::encode(signature.to_bytes()),
        &timestamp_str,
        &nonce_str,
        method,
        path,
        tampered_body,
    );

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AuthError::InvalidSignature(_)
    ));
}

#[test]
fn test_verify_expired_timestamp() {
    let (identity, pubkey) = create_test_identity();
    // Timestamp from 10 minutes ago
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap() - (10 * 60 * 1_000_000_000);
    let nonce = uuid::Uuid::new_v4();
    let method = "POST";
    let path = "/api/v1/users/abc123/profile";
    let body = b"{}";

    let timestamp_str = timestamp.to_string();
    let nonce_str = nonce.to_string();
    let mut message = timestamp_str.as_bytes().to_vec();
    message.extend_from_slice(nonce_str.as_bytes());
    message.extend_from_slice(method.as_bytes());
    message.extend_from_slice(path.as_bytes());
    message.extend_from_slice(body);

    let signature = identity.sign(&message).unwrap();

    let result = verify_request_signature(
        &hex::encode(&pubkey),
        &hex::encode(signature.to_bytes()),
        &timestamp_str,
        &nonce_str,
        method,
        path,
        body,
    );

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AuthError::TimestampExpired));
}

#[test]
fn test_verify_invalid_pubkey_length() {
    let result = verify_request_signature(
        "deadbeef", // Too short
        &hex::encode([0u8; 64]),
        "1234567890",
        "550e8400-e29b-41d4-a716-446655440000",
        "POST",
        "/test",
        b"{}",
    );

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AuthError::InvalidFormat(_)));
}

#[test]
fn test_verify_invalid_signature_length() {
    let (_, pubkey) = create_test_identity();
    let result = verify_request_signature(
        &hex::encode(&pubkey),
        "deadbeef", // Too short
        "1234567890",
        "550e8400-e29b-41d4-a716-446655440000",
        "POST",
        "/test",
        b"{}",
    );

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AuthError::InvalidFormat(_)));
}

#[test]
fn test_verify_invalid_nonce_format() {
    let (_, pubkey) = create_test_identity();
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();
    let result = verify_request_signature(
        &hex::encode(&pubkey),
        &hex::encode([0u8; 64]),
        &timestamp.to_string(),
        "not-a-uuid", // Invalid nonce
        "POST",
        "/test",
        b"{}",
    );

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AuthError::InvalidFormat(_)));
}

#[test]
fn export_typescript_types() {
    SignedRequestHeaders::export().expect("Failed to export SignedRequestHeaders type");
}
