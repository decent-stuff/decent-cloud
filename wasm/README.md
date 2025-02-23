# @decent-stuff/dc-client

NPM package for cloning and interacting with Decent Cloud ledger in the browser. It allows you to clone the complete ledger contents and to iterate over the contents locally. It also allows you to export the ledger contents as JSON.

## Installation

```bash
npm install @decent-stuff/dc-client
```

## Requirements

- Node.js >= 16.0.0
- Modern browser with WebAssembly support

## Features

- Local ledger cloning and storage
- JSON export capabilities
- Full TypeScript support
- Browser-optimized WebAssembly implementation
- Persistent storage management
- Generic canister query and update methods

## Usage

```javascript
import { initialize, ledger_get_block_as_json } from '@decent-stuff/dc-client';

// Initialize the WASM module and storage
await initialize();

// Get a block as JSON
const blockResult = await ledger_get_block_as_json(BigInt(0));
if ('Ok' in blockResult) {
  const { block_header, block } = JSON.parse(blockResult.Ok);
  console.log('Block header:', block_header);
  console.log('Block contents:', block);
}
```

## API Reference

### Core Functions

#### `initialize(): Promise<string>`

Initialize the WASM module, storage system, and LedgerMap. Must be called before using other functions.

#### `ledger_storage_clear(): void`

Clear the storage system.

#### `ledger_get_value(key: Uint8Array): Uint8Array | null`

Get a value from the ledger.

#### `ledger_set_value(key: Uint8Array, value: Uint8Array): void`

Set a value in the ledger.

#### `ledger_remove_value(key: Uint8Array): void`

Remove a value from the ledger.

#### `ledger_get_block_as_json(block_offset: bigint): { Ok: string } | { Err: string }`

Get a ledger block as JSON at the specified offset.

### Canister Interaction

#### `call_query_canister(method_name: string, arg: any): Promise<any>`

Generic query function that can be used for any query method.

#### `call_update_canister(method_name: string, arg: any, identity: any): Promise<any>`

Generic update function that can be used for any update method.

#### `get_transactions(): Promise<any>`

Get transactions from the ledger.

## Development

### Building

```bash
npm run build
```

### Testing

```bash
npm test
```

### Demo

A demo implementation is available in the `demo` directory. To run it:

```bash
cd demo
npm install
npm run dev
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.
