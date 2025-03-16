# Decent Cloud WASM Library

A WebAssembly-powered client library for querying and managing Decent Cloud ledger data in the browser. This library provides efficient binary data processing, local caching via IndexedDB, and a clean TypeScript API.

## Features

- 🚀 High-performance WASM-based ledger data processing
- 💾 Local caching with IndexedDB for offline access
- 🔄 Automatic synchronization with Decent Cloud ledger
- 📦 Full TypeScript support
- 🛠️ Clean, modular API
- 🔍 Detailed block and entry inspection
- 🏗️ Browser-based querying and analysis

## Installation

```bash
npm install @decent-stuff/dc-client
```

## Usage

### Basic Usage

```typescript
import { DecentCloudClient, decentCloudLedger } from '@decent-stuff/dc-client';

// Initialize the client
const client = new DecentCloudClient();
await client.initialize();

// Fetch new ledger blocks
const fetchResult = await decentCloudLedger.fetchLedgerBlocks();
console.log('Fetch result:', fetchResult);

// Get the last fetched block
const lastBlock = await decentCloudLedger.getLastFetchedBlock();
if (lastBlock) {
  console.log('Last block:', lastBlock);

  // Get entries for this block
  const entries = await decentCloudLedger.getBlockEntries(lastBlock.blockOffset);
  console.log('Block entries:', entries);
}

// Clear local storage if needed
await decentCloudLedger.clearStorage();
```

### API Reference

#### DecentCloudClient

The main client class, primarily used for initialization:

```typescript
class DecentCloudClient {
  async initialize(): Promise<void>;
  async clearStorage(): Promise<void>;
}
```

#### DecentCloudLedger

Static methods for ledger operations:

```typescript
class DecentCloudLedger {
  static async fetchLedgerBlocks(): Promise<string>;
  static async getAllEntries(): Promise<LedgerEntry[]>;
  static async getBlockEntries(blockOffset: number): Promise<LedgerEntry[]>;
  static async getLastFetchedBlock(): Promise<LedgerBlock | null>;
  static async clearStorage(): Promise<void>;
}
```

#### Types

```typescript
interface LedgerBlock {
  blockVersion: number;
  blockSize: number;
  parentBlockHash: string;
  blockOffset: number;
  fetchCompareBytes: string;
  fetchOffset: number;
  timestampNs: bigint;
}

interface LedgerEntry {
  blockOffset: number;
  label: string;
  key: string;
  value?: string;
  description?: string;
}
```

## Architecture

The library consists of several key components:

- **Client (dc-client.js)**: Main entry point and WASM initialization
- **Ledger (ledger.ts)**: Core ledger operations and data processing
- **Database (db.ts)**: IndexedDB interface using Dexie.js
- **Agent (agent.ts)**: Decent Cloud ledger communication
- **WASM Module**: High-performance binary data processing

The library downloads Decent Cloud ledger data and enables efficient local querying and analysis directly in the browser. All data is cached in IndexedDB for offline access.

## Demo

A demo application showcasing the library's capabilities is included in the `demo` directory:

```bash
cd demo
npm install
npm run dev
```

## Development

### Building

```bash
npm run build
```

This will:

1. Compile Rust code to WebAssembly
2. Generate TypeScript definitions
3. Bundle JavaScript modules
4. Prepare distribution files

### Testing

```bash
npm test          # Run all tests
npm run test:browser  # Run browser-specific tests
```

## Database

The library uses [Dexie.js](https://dexie.org/) for IndexedDB operations, providing:

- Robust offline storage
- Efficient querying
- Transaction support
- Schema versioning

See [DB_ALTERNATIVES.md](./DB_ALTERNATIVES.md) for details on database selection.

## License

Apache-2.0
