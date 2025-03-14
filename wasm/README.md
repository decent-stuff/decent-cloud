# Decent Cloud WASM Library

This library provides a WebAssembly-based client for interacting with the Decent Cloud ledger. It allows fetching ledger blocks from the remote ledger, processing them using WebAssembly, and storing them in a local IndexedDB database.

## Features

- Fetch ledger blocks from the remote ledger
- Process binary data using WebAssembly
- Store ledger data in IndexedDB for offline access
- TypeScript support
- Clean, modular API

## Installation

```bash
npm install @decent-stuff/dc-client
```

## Usage

### Basic Usage

```javascript
import { createClient } from '@decent-stuff/dc-client';

// Create a client instance
const client = createClient();

// Initialize the client
await client.initialize();

// Fetch ledger blocks
const newBlocksCount = await client.ledger.fetchLedgerBlocks();
console.log(`Fetched ${newBlocksCount} new blocks`);

// Get the last fetched block
const lastBlock = await client.ledger.getLastFetchedBlock();
console.log('Last fetched block:', lastBlock);

// Clear the ledger storage
await client.storage.clear();
```

### Configuration

You can configure the client with custom settings:

```javascript
import { createClient } from '@decent-stuff/dc-client';

// Create a client instance with custom configuration
const client = createClient({
  networkUrl: 'https://custom-icp-api.io',
  canisterId: 'your-canister-id',
});

// Initialize the client
await client.initialize();
```

### Advanced Usage

For more advanced usage, you can access the underlying components directly:

```javascript
import {
  initialize,
  fetchLedgerBlocks,
  getLastFetchedBlock,
  clearLedgerData,
  db,
} from '@decent-stuff/dc-client';

// Initialize the library
await initialize();

// Fetch ledger blocks
const newBlocksCount = await fetchLedgerBlocks();

// Get all ledger entries
const allEntries = await db.getAllEntries();

// Get a specific entry
const entry = await db.getEntry('your-entry-key');

// Clear all data
await clearLedgerData();
```

## Architecture

The library is organized into several modules:

- **client.js**: Main entry point and API
- **wasm.ts**: WebAssembly interface
- **ledger.ts**: Ledger operations
- **db.ts**: Database operations
- **agent.ts**: Internet Computer agent

## Demo

A demo application is included in the `demo` directory. To run it:

```bash
cd demo
npm install
npm run dev
```

Then open your browser to the URL shown in the console.

## Building

To build the library:

```bash
npm run build
```

This will:

1. Compile the Rust code to WebAssembly
2. Bundle the JavaScript files
3. Generate TypeScript definitions

## Database

The library uses [Dexie.js](https://dexie.org/) for IndexedDB access. See [DB_ALTERNATIVES.md](./DB_ALTERNATIVES.md) for an evaluation of alternatives.

## License

Apache-2.0
