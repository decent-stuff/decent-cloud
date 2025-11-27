# LedgerMap

[![Crates.io](https://img.shields.io/crates/v/ledger-map.svg)](https://crates.io/crates/ledger-map)
[![npm](https://img.shields.io/npm/v/@decent-stuff/ledger-map.svg)](https://www.npmjs.com/package/@decent-stuff/ledger-map)
[![Documentation](https://docs.rs/ledger-map/badge.svg)](https://docs.rs/ledger-map)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

A secure, persistent key-value storage system with blockchain-like properties, implemented in Rust and compiled to WebAssembly.

## Key Features

- 🔒 **Secure Storage**: Data integrity protected with SHA-256 checksums
- 📝 **Append-Only Ledger**: Blockchain-like data structure
- 🔄 **Cross-Platform**: Native support for `wasm32`, `x86_64`, and `aarch64`
- 🏷️ **Label Support**: Organize data with multiple labels
- 📦 **TypeScript Support**: First-class TypeScript definitions

## Quick Start

### Rust Projects

```toml
# Cargo.toml
[dependencies]
ledger-map = "0.4.3"
```

```rust
use ledger_map::LedgerMap;

// Create a new ledger map
let mut map = LedgerMap::new_with_path(None, None)
    .expect("Failed to create LedgerMap");

// Store data
map.upsert("users", b"alice".to_vec(), b"data".to_vec())
    .unwrap();
map.commit_block().unwrap();

// Retrieve data
let value = map.get("users", &b"alice".to_vec());
```

### Web/TypeScript Projects

```bash
npm install @decent-stuff/ledger-map
```

```typescript
import { WasmLedgerMap } from "@decent-stuff/ledger-map";

// Initialize
const ledger = new WasmLedgerMap();
await ledger.initialize();

// Store data
const key = new TextEncoder().encode("alice");
const value = new TextEncoder().encode("data");

ledger.beginBlock();
ledger.upsert("users", key, value);
ledger.commitBlock();

// Retrieve data
const retrieved = ledger.get("users", key);
```

## Installation

### Rust

Add to your `Cargo.toml`:

```toml
[dependencies]
ledger-map = "0.4.3"
```

For specific features:

```toml
# For Internet Computer support
ledger-map = { version = "0.4.3", features = ["ic"] }
```

### Web/TypeScript

```bash
# Using npm
npm install @decent-stuff/ledger-map

# Using yarn
yarn add @decent-stuff/ledger-map

# Using pnpm
pnpm add @decent-stuff/ledger-map
```

## Detailed Usage

### Rust API

```rust
use ledger_map::{LedgerMap};
use env_logger::Env;

fn main() {
    // Initialize logging (optional)
    env_logger::try_init_from_env(Env::default().default_filter_or("info")).unwrap();

    // Create LedgerMap with optional custom storage path
    let mut ledger_map = LedgerMap::new_with_path(
        None,  // No specific labels to index
        None   // Use default storage path
    ).expect("Failed to create LedgerMap");

    // Store data with labels
    ledger_map.upsert("users", b"alice".to_vec(), b"data1".to_vec()).unwrap();
    ledger_map.upsert("posts", b"post1".to_vec(), b"content".to_vec()).unwrap();
    ledger_map.commit_block().unwrap();

    // Query by label
    let user_entries = ledger_map.iter(Some("users")).collect::<Vec<_>>();
    let post_entries = ledger_map.iter(Some("posts")).collect::<Vec<_>>();

    // Delete data
    ledger_map.delete("users", b"alice".to_vec()).unwrap();
    ledger_map.commit_block().unwrap();
}
```

### TypeScript API

```typescript
import { WasmLedgerMap } from "@decent-stuff/ledger-map";

async function example() {
  // Initialize
  const ledger = new WasmLedgerMap();
  await ledger.initialize(["users", "posts"]); // Pre-index labels

  // Store data
  const key = new TextEncoder().encode("user1");
  const value = new TextEncoder().encode(JSON.stringify({ name: "Alice" }));

  ledger.beginBlock();
  ledger.upsert("users", key, value);
  ledger.commitBlock();

  // Retrieve data
  const retrieved = ledger.get("users", key);
  const userData = JSON.parse(new TextDecoder().decode(retrieved));

  // Delete data
  ledger.beginBlock();
  ledger.delete("users", key);
  ledger.commitBlock();
}
```

## API Reference

### Rust API

- `LedgerMap::new()` - Create a new ledger map with default settings
- `LedgerMap::new_with_path(labels: Option<&[&str]>, path: Option<PathBuf>)` - Create with custom settings
- `upsert(label: &str, key: Vec<u8>, value: Vec<u8>)` - Store or update a value
- `get(label: &str, key: &[u8]) -> Option<&Vec<u8>>` - Retrieve a value
- `delete(label: &str, key: Vec<u8>)` - Delete a value
- `commit_block()` - Commit pending changes
- `iter(label: Option<&str>)` - Iterate over entries

### TypeScript API

- `initialize(labels?: string[])` - Initialize the ledger
- `upsert(label: string, key: Uint8Array, value: Uint8Array)` - Store or update a value
- `get(label: string, key: Uint8Array)` - Retrieve a value
- `delete(label: string, key: Uint8Array)` - Delete a value
- `beginBlock()` - Start a new block of operations
- `commitBlock()` - Commit the current block
- `getBlocksCount()` - Get total number of blocks
- `getLatestBlockHash()` - Get latest block hash
- `refreshLedger()` - Reload from storage

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details on how to:

- Set up the development environment
- Run tests
- Submit pull requests
- Follow our coding standards

## Testing

### Rust Tests

```bash
cargo test
cargo test --features ic
```

### TypeScript Tests

```bash
# Node.js tests
npm run test
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
