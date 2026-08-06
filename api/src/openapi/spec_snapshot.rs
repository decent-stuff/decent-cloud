//! OpenAPI spec stability guard.
//!
//! Renders the full combined OpenAPI spec (`create_combined_api`), canonicalizes
//! it (recursive JSON key sort so emission order is irrelevant), and asserts a
//! stable content hash + path/schema counts. This codifies the invariant that
//! every `*Api` split (issue #444) leaves the spec **byte-identical**: if a
//! refactor accidentally adds/removes/reorders an endpoint or schema, this test
//! goes red.
//!
//! Note: raw `Route::at(..)` handlers that are NOT `#[OpenApi]` impls (e.g. the
//! inbound Stripe/Chatwoot/Telegram webhooks in `webhooks.rs`) do not appear in
//! this spec at all — moving their internal helpers between modules therefore
//! cannot change it.

use poem_openapi::OpenApiService;

/// Recursively sort every JSON object's keys so the serialized form is
/// order-independent (poem-openapi emits `paths` keys in tuple-registration
/// order, which is cosmetic). Arrays keep their order (parameters/tags are
/// order-sensitive).
fn canonicalize(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            // serde_json::Map preserves insertion order; collect+reinsert sorted.
            let sorted: std::collections::BTreeMap<String, serde_json::Value> =
                std::mem::take(map).into_iter().collect();
            for (k, mut v) in sorted {
                canonicalize(&mut v);
                map.insert(k, v);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                canonicalize(item);
            }
        }
        _ => {}
    }
}

fn render_canonical_spec() -> serde_json::Value {
    let service =
        OpenApiService::new(super::create_combined_api(), "Decent Cloud API", "1.0.0")
            .server("/api/v1");
    let mut spec: serde_json::Value =
        serde_json::from_str(&service.spec()).expect("combined OpenAPI spec must parse as JSON");
    canonicalize(&mut spec);
    spec
}

#[test]
fn openapi_spec_is_stable() {
    let spec = render_canonical_spec();

    // Optional dump for empirical before/after diffing during a split:
    //   DC_OPENAPI_SPEC_DUMP=/tmp/spec.json cargo test openapi_spec_is_stable
    if let Ok(path) = std::env::var("DC_OPENAPI_SPEC_DUMP") {
        let pretty = serde_json::to_string_pretty(&spec).expect("pretty print");
        std::fs::write(&path, pretty).unwrap_or_else(|e| panic!("write {path}: {e:#}"));
    }

    let paths = spec["paths"]
        .as_object()
        .map(|m| m.len())
        .expect("spec must have a paths object");
    let schemas = spec["components"]["schemas"]
        .as_object()
        .map(|m| m.len())
        .unwrap_or(0);

    let canonical = serde_json::to_string(&spec).expect("canonical serialize");
    let hash = sha2_256_hex(canonical.as_bytes());

    eprintln!(
        "DC_OPENAPI_SPEC: paths={paths} schemas={schemas} hash={hash} bytes={}",
        canonical.len()
    );

    // Snapshot captured from the live combined API. Update ONLY as part of an
    // intentional, verified spec change — never silently.
    //
    // Capture date: 2026-08-06. Refreshed after removing the dead `is_example`
    // field from the `Offering` schema (migration 053 dropped all example-provider
    // seed data, so `is_example` was always false). Path/schema counts are
    // unchanged (187/327); the only diff is the removal of the `is_example`
    // property + its `required` entry on `Offering`.
    const EXPECTED_PATHS: usize = 187;
    const EXPECTED_SCHEMAS: usize = 327;
    const EXPECTED_HASH: &str = "f4b3cfd966aa0e7250a9ac75023b71d0a19ca3ee6f433a5532932c2d61d5c843";

    assert_eq!(paths, EXPECTED_PATHS, "OpenAPI path count drifted");
    assert_eq!(schemas, EXPECTED_SCHEMAS, "OpenAPI schema count drifted");
    assert_eq!(
        hash, EXPECTED_HASH,
        "OpenAPI canonical spec hash drifted — the spec changed; \
         run with DC_OPENAPI_SPEC_DUMP to inspect"
    );
}

/// Minimal SHA-256 hex (avoids pulling a second hasher into the test dep graph;
/// `sha2` + `hex` are already workspace deps via stripe/signature paths).
fn sha2_256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
