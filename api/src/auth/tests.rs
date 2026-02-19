use super::*;
use dcc_common::DccIdentity;
use ts_rs::TS;

fn create_test_identity() -> (DccIdentity, Vec<u8>) {
    let seed = [42u8; 32];
    let identity =
        DccIdentity::new_from_seed(&seed).expect("Failed to create test identity from seed");
    let pubkey = identity.to_bytes_verifying();
    (identity, pubkey)
}

/// Test vector for cross-platform signature verification.
/// TypeScript frontend must produce the same signature.
#[test]
fn test_cross_platform_signature_vector() {
    // Fixed 32-byte seed (used directly, not via HMAC - matches Ed25519KeyIdentity.fromSecretKey)
    let seed: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ];
    // Create identity directly from seed bytes (not via HMAC derivation)
    let identity = DccIdentity::new_signing_from_bytes(&seed)
        .expect("Failed to create identity from seed bytes");
    let pubkey = identity.to_bytes_verifying();

    // Fixed message
    let message = b"test message for cross-platform verification";

    // Sign
    let signature = identity.sign(message).expect("Failed to sign message");

    // Print test vector
    println!("=== Cross-platform test vector ===");
    println!("Seed (hex): {}", hex::encode(seed));
    println!("Public key (hex): {}", hex::encode(&pubkey));
    println!("Message: {}", String::from_utf8_lossy(message));
    println!("Signature (hex): {}", hex::encode(signature.to_bytes()));

    // Verify signature works
    identity
        .verify(message, &signature)
        .expect("Failed to verify signature");

    // Expected values - update TypeScript test if these change
    let expected_pubkey = "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8";
    let expected_signature = "a2aca8ef6760241fc2b254447b9320f03fffaaa11f60365b33455b5d664abc0172627ce2258cdbde7e2eddbe20bda46e008f8041ffb61515e7f4e5a8fdab3f0f";
    assert_eq!(hex::encode(&pubkey), expected_pubkey, "Public key mismatch");
    assert_eq!(
        hex::encode(signature.to_bytes()),
        expected_signature,
        "Signature mismatch"
    );
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
    let signature = identity.sign(&message).expect("Failed to sign message");

    // Verify
    let result = verify_request_signature(
        &hex::encode(&pubkey),
        &hex::encode(signature.to_bytes()),
        &timestamp_str,
        &nonce_str,
        method,
        path,
        body,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        pubkey,
        "Verified public key should match original public key"
    );
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
    let signature = identity.sign(&message).expect("Failed to sign message");

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
        None,
    );

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), AuthError::InvalidSignature(_)),
        "Error should be InvalidSignature for tampered message"
    );
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
        None,
    );

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), AuthError::TimestampExpired),
        "Error should be TimestampExpired"
    );
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
        None,
    );

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), AuthError::InvalidFormat(_)),
        "Error should be InvalidFormat for invalid public key length"
    );
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
        None,
    );

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), AuthError::InvalidFormat(_)),
        "Error should be InvalidFormat for invalid signature length"
    );
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
        None,
    );

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), AuthError::InvalidFormat(_)),
        "Error should be InvalidFormat for invalid nonce format"
    );
}

#[test]
fn export_typescript_types() {
    SignedRequestHeaders::export().expect("Failed to export SignedRequestHeaders type");
}

#[test]
fn test_actual_failing_signature_from_curl() {
    // Exact values from the failing curl request the user reported
    let pubkey_hex = "4f16b64a096a48252fc4d9a11393778d84e95fcf6868b72b9d2b344d4b103386";
    let signature_hex = "e6cd8399dffffbf4971415fcb3e27600b1377b20d3fcfee6ac6a16cf47129ce63ebb38b7a8f9e73e231a1af1aa48261a34218461d533087a50ae927ebc88ce09";
    let timestamp = "1763841226767000000";
    let nonce = "613a7ce4-e8b7-45d2-9b6d-04f6208f6093";
    let method = "GET";
    let path = "/api/v1/provider/rental-requests/pending";
    let body = b""; // Empty for GET request
    let now_ns = timestamp
        .parse::<i64>()
        .expect("Timestamp should be valid i64 nanoseconds");

    println!("\n=== Verifying actual failing signature ===");
    println!("Public key: {}", pubkey_hex);
    println!("Timestamp: {}", timestamp);
    println!("Nonce: {}", nonce);
    println!("Method: {}", method);
    println!("Path: {}", path);
    println!("Body length: {}", body.len());

    let message = format!("{}{}{}{}", timestamp, nonce, method, path);
    println!("Message (no body): {}", message);
    println!("Message length: {}", message.len());

    let result = verify_request_signature(
        pubkey_hex,
        signature_hex,
        timestamp,
        nonce,
        method,
        path,
        body,
        Some(now_ns),
    );

    match &result {
        Ok(verified_pubkey) => {
            println!("✓ Signature VALID");
            println!("Verified pubkey: {}", hex::encode(verified_pubkey));
        }
        Err(e) => {
            println!("✗ Signature INVALID: {}", e);
        }
    }

    // Fail if signature doesn't verify - this documents the bug
    result.expect("Signature should be valid but is failing");
}

#[test]
fn test_signature_crypto_only() {
    // This test verifies ONLY the cryptographic signature, not the timestamp
    // Using values from user's request
    let pubkey_hex = "4f16b64a096a48252fc4d9a11393778d84e95fcf6868b72b9d2b344d4b103386";
    let signature_hex = "e6cd8399dffffbf4971415fcb3e27600b1377b20d3fcfee6ac6a16cf47129ce63ebb38b7a8f9e73e231a1af1aa48261a34218461d533087a50ae927ebc88ce09";
    let timestamp_str = "1763841226767000000";
    let nonce_str = "613a7ce4-e8b7-45d2-9b6d-04f6208f6093";
    let method = "GET";
    let path = "/api/v1/provider/rental-requests/pending";
    let body = b"";

    // Decode public key and signature
    let pubkey = hex::decode(pubkey_hex).expect("Failed to decode public key from hex");
    let signature = hex::decode(signature_hex).expect("Failed to decode signature from hex");

    // Construct message: timestamp + nonce + method + path + body
    let mut message = timestamp_str.as_bytes().to_vec();
    message.extend_from_slice(nonce_str.as_bytes());
    message.extend_from_slice(method.as_bytes());
    message.extend_from_slice(path.as_bytes());
    message.extend_from_slice(body);

    println!("\n=== Testing cryptographic signature only ===");
    println!("Message length: {}", message.len());
    println!("Public key length: {}", pubkey.len());
    println!("Signature length: {}", signature.len());

    // Verify signature using DccIdentity
    let identity =
        DccIdentity::new_verifying_from_bytes(&pubkey).expect("Failed to create identity");

    let verify_result = identity.verify_bytes(&message, &signature);

    match &verify_result {
        Ok(()) => println!("✓ Signature cryptographically VALID"),
        Err(e) => println!("✗ Signature cryptographically INVALID: {}", e),
    }

    verify_result.expect("Cryptographic signature should be valid");
}
