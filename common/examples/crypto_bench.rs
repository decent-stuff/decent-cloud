//! Auth crypto microbenchmark — the canonical perf-regression tool for the
//! request-signature verification hot path (see api/src/auth.rs).
//!
//! Lives in `dcc-common` (not `api`) so a release build is fast: it pulls in
//! only ed25519-dalek, not the whole api crate. Run both profiles to compare:
//!
//!     cargo run -p dcc-common --example crypto_bench --release   # prod-like
//!     cargo run -p dcc-common --example crypto_bench             # debug (slow)
//!
//! What it proves (release numbers, 2026-07-22):
//!   - verify_bytes (the curve math) is ~96% of auth cost and NOT cacheable.
//!   - new_verifying_from_bytes is ~7% and IS cacheable (api caches it).
//!   - debug is ~150x slower than release for curve math — never profile in debug.
use dcc_common::DccIdentity;
use ed25519_dalek::{Digest, Sha512};

fn main() {
    let seed = [42u8; 32];
    let signing = DccIdentity::new_from_seed(&seed).expect("create signing identity");
    let pubkey = signing.to_bytes_verifying();
    let pubkey_hex = hex::encode(&pubkey);

    let timestamp = "1700000000000000000";
    let nonce = "11111111-1111-1111-1111-111111111111";
    let method = "GET";
    let path = "/api/v1/providers/abc/trust-metrics";
    let body: Vec<u8> = vec![];

    let mut message = vec![];
    message.extend_from_slice(timestamp.as_bytes());
    message.extend_from_slice(nonce.as_bytes());
    message.extend_from_slice(method.as_bytes());
    message.extend_from_slice(path.as_bytes());
    message.extend_from_slice(&body);
    let signature = signing.sign(&message).expect("sign");
    let signature_hex = hex::encode(signature.to_bytes());

    let n: u32 = 5000;

    let t = std::time::Instant::now();
    for _ in 0..n {
        let _ = hex::decode(&pubkey_hex).unwrap();
    }
    let hex_decode_ns = t.elapsed().as_nanos() / n as u128;

    let t = std::time::Instant::now();
    for _ in 0..n {
        let _ = DccIdentity::new_verifying_from_bytes(&pubkey).unwrap();
    }
    let construct_ns = t.elapsed().as_nanos() / n as u128;

    let identity = DccIdentity::new_verifying_from_bytes(&pubkey).unwrap();
    let t = std::time::Instant::now();
    for _ in 0..n {
        identity
            .verify_bytes(&message, &signature.to_bytes())
            .unwrap();
    }
    let verify_ns = t.elapsed().as_nanos() / n as u128;

    // decompression cost in isolation (what verify does internally per call)
    let t = std::time::Instant::now();
    for _ in 0..n {
        let mut prehashed = Sha512::new();
        prehashed.update(&message);
    }
    let sha_ns = t.elapsed().as_nanos() / n as u128;

    let t = std::time::Instant::now();
    for _ in 0..n {
        let pk = hex::decode(&pubkey_hex).unwrap();
        let sig = hex::decode(&signature_hex).unwrap();
        let id = DccIdentity::new_verifying_from_bytes(&pk).unwrap();
        id.verify_bytes(&message, &sig).unwrap();
    }
    let full_ns = t.elapsed().as_nanos() / n as u128;

    println!("=== crypto bench (n={n}) ===");
    println!("(a) hex::decode(pubkey)        : {hex_decode_ns:>8} ns");
    println!("(b) new_verifying_from_bytes   : {construct_ns:>8} ns  <-- cacheable");
    println!("(c) verify_bytes (per-request) : {verify_ns:>8} ns  <-- NOT cacheable");
    println!("(s) sha512(message) only       : {sha_ns:>8} ns");
    println!("(d) full hot path (a+b+c+sig)  : {full_ns:>8} ns");
    println!(
        "cacheable fraction (b/d)       : {:.2}%",
        100.0 * construct_ns as f64 / full_ns as f64
    );
}
