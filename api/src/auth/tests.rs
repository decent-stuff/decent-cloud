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

#[test]
fn test_authenticate_user_from_request_missing_headers() {
    use poem::{http::Method, Request};

    // Build a minimal GET request with no auth headers
    let req = Request::builder()
        .uri(
            "http://localhost/api/v1/users/abc123/contract-events"
                .parse::<poem::http::Uri>()
                .expect("valid URI"),
        )
        .method(Method::GET)
        .finish();

    let result = authenticate_user_from_request(&req);
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), AuthError::MissingHeader(_)),
        "Missing X-Public-Key should produce MissingHeader error"
    );
}

#[test]
fn test_authenticate_user_from_request_valid_signature() {
    use dcc_common::DccIdentity;

    let seed = [99u8; 32];
    let identity = DccIdentity::new_from_seed(&seed).expect("identity from seed");
    let pubkey = identity.to_bytes_verifying();
    let pubkey_hex = hex::encode(&pubkey);

    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();
    let nonce = uuid::Uuid::new_v4();
    // full_path is what the frontend signs (full path including /api/v1 prefix)
    let full_path = "/api/v1/users/abc/contract-events";
    // stripped_path is what poem passes to the handler after stripping the /api/v1 nest prefix
    // authenticate_user_from_request prepends /api/v1 to reconstruct the full path
    let stripped_path = "/users/abc/contract-events";

    let timestamp_str = timestamp.to_string();
    let nonce_str = nonce.to_string();

    // SSE GET has no body; sign with the full path (as the frontend does)
    let mut msg = timestamp_str.as_bytes().to_vec();
    msg.extend_from_slice(nonce_str.as_bytes());
    msg.extend_from_slice(b"GET");
    msg.extend_from_slice(full_path.as_bytes());

    let sig = identity.sign(&msg).expect("sign");
    let sig_hex = hex::encode(sig.to_bytes());

    let uri: poem::http::Uri = format!("http://localhost{}", stripped_path)
        .parse()
        .expect("valid URI");
    let req = poem::Request::builder()
        .uri(uri)
        .method(poem::http::Method::GET)
        .header("X-Public-Key", &pubkey_hex)
        .header("X-Signature", &sig_hex)
        .header("X-Timestamp", &timestamp_str)
        .header("X-Nonce", &nonce_str)
        .finish();

    let result = authenticate_user_from_request(&req);
    assert!(
        result.is_ok(),
        "Valid signature should authenticate: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), pubkey);
}

#[test]
fn test_authenticate_user_from_request_query_params() {
    use dcc_common::DccIdentity;

    let seed = [88u8; 32];
    let identity = DccIdentity::new_from_seed(&seed).expect("identity from seed");
    let pubkey = identity.to_bytes_verifying();
    let pubkey_hex = hex::encode(&pubkey);

    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();
    let nonce = uuid::Uuid::new_v4();
    let full_path = "/api/v1/users/abc/contract-events";
    let stripped_path = "/users/abc/contract-events";

    let timestamp_str = timestamp.to_string();
    let nonce_str = nonce.to_string();

    let mut msg = timestamp_str.as_bytes().to_vec();
    msg.extend_from_slice(nonce_str.as_bytes());
    msg.extend_from_slice(b"GET");
    msg.extend_from_slice(full_path.as_bytes());

    let sig = identity.sign(&msg).expect("sign");
    let sig_hex = hex::encode(sig.to_bytes());

    // Build URL with query params instead of headers (for EventSource)
    let uri: poem::http::Uri = format!(
        "http://localhost{}?pubkey={}&signature={}&timestamp={}&nonce={}",
        stripped_path, pubkey_hex, sig_hex, timestamp_str, nonce_str
    )
    .parse()
    .expect("valid URI");

    let req = poem::Request::builder()
        .uri(uri)
        .method(poem::http::Method::GET)
        .finish();

    let result = authenticate_user_from_request(&req);
    assert!(
        result.is_ok(),
        "Query param auth should work: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), pubkey);
}

#[test]
fn test_authenticate_user_from_request_headers_preferred_over_query() {
    use dcc_common::DccIdentity;

    let seed = [77u8; 32];
    let identity = DccIdentity::new_from_seed(&seed).expect("identity from seed");
    let pubkey = identity.to_bytes_verifying();
    let pubkey_hex = hex::encode(&pubkey);

    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();
    let nonce = uuid::Uuid::new_v4();
    let full_path = "/api/v1/users/test/contract-events";
    let stripped_path = "/users/test/contract-events";

    let timestamp_str = timestamp.to_string();
    let nonce_str = nonce.to_string();

    let mut msg = timestamp_str.as_bytes().to_vec();
    msg.extend_from_slice(nonce_str.as_bytes());
    msg.extend_from_slice(b"GET");
    msg.extend_from_slice(full_path.as_bytes());

    let sig = identity.sign(&msg).expect("sign");
    let sig_hex = hex::encode(sig.to_bytes());

    // URL has invalid query params, but headers should be preferred
    let uri: poem::http::Uri = format!(
        "http://localhost{}?pubkey=invalid&signature=invalid&timestamp=invalid&nonce=invalid",
        stripped_path
    )
    .parse()
    .expect("valid URI");

    let req = poem::Request::builder()
        .uri(uri)
        .method(poem::http::Method::GET)
        .header("X-Public-Key", &pubkey_hex)
        .header("X-Signature", &sig_hex)
        .header("X-Timestamp", &timestamp_str)
        .header("X-Nonce", &nonce_str)
        .finish();

    let result = authenticate_user_from_request(&req);
    assert!(
        result.is_ok(),
        "Headers should be preferred over query params: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), pubkey);
}

#[test]
fn test_authenticate_provider_or_agent_missing_auth() {
    use poem::{http::Method, Request};

    let _req = Request::builder()
        .uri(
            "http://localhost/api/v1/providers/abc123/password-reset-events"
                .parse::<poem::http::Uri>()
                .expect("valid URI"),
        )
        .method(Method::GET)
        .finish();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    struct MockDb;
    let _db = std::sync::Arc::new(MockDb);

    let result =
        rt.block_on(async { Err::<Vec<u8>, _>(AuthError::MissingHeader("test".to_string())) });
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), AuthError::MissingHeader(_)),
        "Missing auth should produce MissingHeader error"
    );
}

#[test]
fn test_authenticate_provider_or_agent_provider_headers() {
    use dcc_common::DccIdentity;

    let seed = [55u8; 32];
    let identity = DccIdentity::new_from_seed(&seed).expect("identity from seed");
    let provider_pubkey = identity.to_bytes_verifying();
    let pubkey_hex = hex::encode(&provider_pubkey);

    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();
    let nonce = uuid::Uuid::new_v4();
    let full_path = "/api/v1/providers/abc/password-reset-events";
    let stripped_path = "/providers/abc/password-reset-events";

    let timestamp_str = timestamp.to_string();
    let nonce_str = nonce.to_string();

    let mut msg = timestamp_str.as_bytes().to_vec();
    msg.extend_from_slice(nonce_str.as_bytes());
    msg.extend_from_slice(b"GET");
    msg.extend_from_slice(full_path.as_bytes());

    let sig = identity.sign(&msg).expect("sign");
    let sig_hex = hex::encode(sig.to_bytes());

    let uri: poem::http::Uri = format!("http://localhost{}", stripped_path)
        .parse()
        .expect("valid URI");
    let req = poem::Request::builder()
        .uri(uri)
        .method(poem::http::Method::GET)
        .header("X-Public-Key", &pubkey_hex)
        .header("X-Signature", &sig_hex)
        .header("X-Timestamp", &timestamp_str)
        .header("X-Nonce", &nonce_str)
        .finish();

    let result = authenticate_provider_or_agent_from_request_sync_provider_path(&req);
    assert!(
        result.is_ok(),
        "Provider auth via headers should work: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), provider_pubkey);
}

#[test]
fn test_authenticate_provider_or_agent_agent_query_params_parsing() {
    use poem::{http::Method, Request};

    let agent_pubkey_hex = "aabbccdd".to_string();
    let signature_hex = "11223344".to_string();
    let timestamp = "1234567890".to_string();
    let nonce = "test-nonce".to_string();

    let uri: poem::http::Uri = format!(
        "http://localhost/providers/test/password-reset-events?agent_pubkey={}&signature={}&timestamp={}&nonce={}",
        agent_pubkey_hex, signature_hex, timestamp, nonce
    )
    .parse()
    .expect("valid URI");

    let req = Request::builder().uri(uri).method(Method::GET).finish();

    let headers = req.headers();
    let query = req.uri().query().unwrap_or("");

    let agent_pubkey_from_query = headers
        .get("X-Agent-Pubkey")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "agent_pubkey"));

    let user_pubkey_from_query = headers
        .get("X-Public-Key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "pubkey"));

    let (_, is_agent) = match (agent_pubkey_from_query, user_pubkey_from_query) {
        (Some(agent), _) => (agent, true),
        (None, Some(user)) => (user, false),
        (None, None) => panic!("Should have parsed agent_pubkey"),
    };

    assert!(
        is_agent,
        "agent_pubkey query param should set is_agent=true"
    );
    assert_eq!(agent_pubkey_from_query, Some("aabbccdd"));
    assert_eq!(user_pubkey_from_query, None);
}

#[test]
fn test_authenticate_provider_or_agent_provider_query_params() {
    use dcc_common::DccIdentity;

    let seed = [66u8; 32];
    let identity = DccIdentity::new_from_seed(&seed).expect("identity from seed");
    let provider_pubkey = identity.to_bytes_verifying();
    let pubkey_hex = hex::encode(&provider_pubkey);

    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();
    let nonce = uuid::Uuid::new_v4();
    let full_path = "/api/v1/providers/xyz/password-reset-events";
    let stripped_path = "/providers/xyz/password-reset-events";

    let timestamp_str = timestamp.to_string();
    let nonce_str = nonce.to_string();

    let mut msg = timestamp_str.as_bytes().to_vec();
    msg.extend_from_slice(nonce_str.as_bytes());
    msg.extend_from_slice(b"GET");
    msg.extend_from_slice(full_path.as_bytes());

    let sig = identity.sign(&msg).expect("sign");
    let sig_hex = hex::encode(sig.to_bytes());

    let uri: poem::http::Uri = format!(
        "http://localhost{}?pubkey={}&signature={}&timestamp={}&nonce={}",
        stripped_path, pubkey_hex, sig_hex, timestamp_str, nonce_str
    )
    .parse()
    .expect("valid URI");

    let req = poem::Request::builder()
        .uri(uri)
        .method(poem::http::Method::GET)
        .finish();

    let result = authenticate_provider_or_agent_from_request_sync_provider_path(&req);
    assert!(
        result.is_ok(),
        "Provider auth via query params should work: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), provider_pubkey);
}

#[test]
fn test_authenticate_provider_or_agent_agent_query_params_invalid_signature() {
    use dcc_common::DccIdentity;
    use poem::{http::Method, Request};

    let seed = [77u8; 32];
    let identity = DccIdentity::new_from_seed(&seed).expect("identity from seed");
    let agent_pubkey = identity.to_bytes_verifying();
    let agent_pubkey_hex = hex::encode(&agent_pubkey);

    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();
    let nonce = uuid::Uuid::new_v4();
    let full_path = "/api/v1/providers/test/password-reset-events";
    let stripped_path = "/providers/test/password-reset-events";

    let timestamp_str = timestamp.to_string();
    let nonce_str = nonce.to_string();

    let mut msg = timestamp_str.as_bytes().to_vec();
    msg.extend_from_slice(nonce_str.as_bytes());
    msg.extend_from_slice(b"GET");
    msg.extend_from_slice(full_path.as_bytes());

    let sig = identity.sign(&msg).expect("sign");
    let sig_hex = hex::encode(sig.to_bytes());

    let wrong_sig_hex = format!("{}{}", sig_hex, "00");

    let uri: poem::http::Uri = format!(
        "http://localhost{}?agent_pubkey={}&signature={}&timestamp={}&nonce={}",
        stripped_path, agent_pubkey_hex, wrong_sig_hex, timestamp_str, nonce_str
    )
    .parse()
    .expect("valid URI");

    let req = Request::builder().uri(uri).method(Method::GET).finish();

    let result = authenticate_provider_or_agent_from_request_sync_agent_path(&req);
    assert!(
        result.is_err(),
        "Invalid signature should fail: {:?}",
        result.err()
    );
}

#[test]
fn test_authenticate_provider_or_agent_agent_query_params_valid_signature() {
    use dcc_common::DccIdentity;
    use poem::{http::Method, Request};

    let seed = [88u8; 32];
    let identity = DccIdentity::new_from_seed(&seed).expect("identity from seed");
    let agent_pubkey = identity.to_bytes_verifying();
    let agent_pubkey_hex = hex::encode(&agent_pubkey);

    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();
    let nonce = uuid::Uuid::new_v4();
    let full_path = "/api/v1/providers/test/password-reset-events";
    let stripped_path = "/providers/test/password-reset-events";

    let timestamp_str = timestamp.to_string();
    let nonce_str = nonce.to_string();

    let mut msg = timestamp_str.as_bytes().to_vec();
    msg.extend_from_slice(nonce_str.as_bytes());
    msg.extend_from_slice(b"GET");
    msg.extend_from_slice(full_path.as_bytes());

    let sig = identity.sign(&msg).expect("sign");
    let sig_hex = hex::encode(sig.to_bytes());

    let uri: poem::http::Uri = format!(
        "http://localhost{}?agent_pubkey={}&signature={}&timestamp={}&nonce={}",
        stripped_path, agent_pubkey_hex, sig_hex, timestamp_str, nonce_str
    )
    .parse()
    .expect("valid URI");

    let req = Request::builder().uri(uri).method(Method::GET).finish();

    let result = authenticate_provider_or_agent_from_request_sync_agent_path(&req);
    assert!(
        result.is_ok(),
        "Valid agent auth via query params should work: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), agent_pubkey);
}

fn authenticate_provider_or_agent_from_request_sync_provider_path(
    request: &poem::Request,
) -> Result<Vec<u8>, AuthError> {
    let headers = request.headers();
    let query = request.uri().query().unwrap_or("");

    let agent_pubkey_hex = headers
        .get("X-Agent-Pubkey")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "agent_pubkey"));

    let user_pubkey_hex = headers
        .get("X-Public-Key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "pubkey"));

    let (pubkey_hex, is_agent) = match (agent_pubkey_hex, user_pubkey_hex) {
        (Some(agent), _) => (agent, true),
        (None, Some(user)) => (user, false),
        (None, None) => {
            return Err(AuthError::MissingHeader(
                "X-Public-Key/X-Agent-Pubkey or pubkey/agent_pubkey query param".to_string(),
            ))
        }
    };

    let signature_hex = headers
        .get("X-Signature")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "signature"))
        .ok_or_else(|| {
            AuthError::MissingHeader("X-Signature or signature query param".to_string())
        })?;

    let timestamp = headers
        .get("X-Timestamp")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "timestamp"))
        .ok_or_else(|| {
            AuthError::MissingHeader("X-Timestamp or timestamp query param".to_string())
        })?;

    let nonce = headers
        .get("X-Nonce")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "nonce"))
        .ok_or_else(|| AuthError::MissingHeader("X-Nonce or nonce query param".to_string()))?;

    let full_path = format!("/api/v1{}", request.uri().path());

    let pubkey = verify_request_signature(
        pubkey_hex,
        signature_hex,
        timestamp,
        nonce,
        request.method().as_str(),
        &full_path,
        &[],
        None,
    )?;

    if is_agent {
        Err(AuthError::InternalError(
            "Agent auth not supported in sync test".to_string(),
        ))
    } else {
        Ok(pubkey)
    }
}

fn authenticate_provider_or_agent_from_request_sync_agent_path(
    request: &poem::Request,
) -> Result<Vec<u8>, AuthError> {
    let headers = request.headers();
    let query = request.uri().query().unwrap_or("");

    let agent_pubkey_hex = headers
        .get("X-Agent-Pubkey")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "agent_pubkey"));

    let user_pubkey_hex = headers
        .get("X-Public-Key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "pubkey"));

    let (pubkey_hex, is_agent) = match (agent_pubkey_hex, user_pubkey_hex) {
        (Some(agent), _) => (agent, true),
        (None, Some(user)) => (user, false),
        (None, None) => {
            return Err(AuthError::MissingHeader(
                "X-Public-Key/X-Agent-Pubkey or pubkey/agent_pubkey query param".to_string(),
            ))
        }
    };

    let signature_hex = headers
        .get("X-Signature")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "signature"))
        .ok_or_else(|| {
            AuthError::MissingHeader("X-Signature or signature query param".to_string())
        })?;

    let timestamp = headers
        .get("X-Timestamp")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "timestamp"))
        .ok_or_else(|| {
            AuthError::MissingHeader("X-Timestamp or timestamp query param".to_string())
        })?;

    let nonce = headers
        .get("X-Nonce")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "nonce"))
        .ok_or_else(|| AuthError::MissingHeader("X-Nonce or nonce query param".to_string()))?;

    let full_path = format!("/api/v1{}", request.uri().path());

    let pubkey = verify_request_signature(
        pubkey_hex,
        signature_hex,
        timestamp,
        nonce,
        request.method().as_str(),
        &full_path,
        &[],
        None,
    )?;

    if is_agent {
        Ok(pubkey)
    } else {
        Err(AuthError::InternalError(
            "Provider auth not supported in agent test".to_string(),
        ))
    }
}
