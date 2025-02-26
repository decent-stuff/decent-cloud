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
- Intuitive class-based API

## Usage

### Class-based API (Recommended)

```javascript
import { createClient } from '@decent-stuff/dc-client';

// Create a client instance with optional configuration
const client = createClient({
  networkUrl: 'https://icp-api.io',
  canisterId: 'ggi4a-wyaaa-aaaai-actqq-cai',
});

// Initialize the client
await client.initialize();

// Get a block as JSON
const blockResult = client.ledger.getBlockAsJson(BigInt(0));
const parsedResult = JSON.parse(blockResult);
const block_header = JSON.parse(parsedResult.block_header);
const block = parsedResult.block;
console.log('Block header:', block_header);
console.log('Block contents:', block);

// Work with ledger values
const label = 'example';
const key = new TextEncoder().encode('test-key');
const value = new TextEncoder().encode('test-value');

// Set a value
client.ledger.setValue(label, key, value);

// Get the value back
const retrievedValue = client.ledger.getValue(label, key);
if (retrievedValue) {
  console.log('Retrieved value:', new TextDecoder().decode(retrievedValue));
}

// Call a canister method
const transactions = await client.canister.callQuery('get_transactions', []);
```

### Function-based API (Legacy)

```javascript
import { initialize, ledger_get_block_as_json } from '@decent-stuff/dc-client';

// Initialize the WASM module and storage
await initialize();

// Get a block as JSON
const blockResult = await ledger_get_block_as_json(BigInt(0));
const parsedResult = JSON.parse(blockResult);
const block_header = JSON.parse(parsedResult.block_header);
const block = parsedResult.block;
console.log('Block header:', block_header);
console.log('Block contents:', block);
```

## API Reference

### Class-based API

#### `createClient(config?: ClientConfig): DecentCloudClient`

Create a new DecentCloudClient instance with optional configuration.

```typescript
interface ClientConfig {
  networkUrl?: string; // Default: 'https://icp-api.io'
  canisterId?: string; // Default: 'ggi4a-wyaaa-aaaai-actqq-cai'
}
```

#### `DecentCloudClient`

Main client class for interacting with Decent Cloud.

##### Methods

- `initialize(): Promise<string>` - Initialize the WASM module, storage system, and LedgerMap.

##### Properties

- `storage: LedgerStorage` - Access to ledger storage operations
- `ledger: LedgerOperations` - Access to ledger data operations
- `canister: CanisterInteraction` - Access to canister interaction methods

#### `LedgerStorage`

Provides methods for managing ledger storage.

- `clear(): void` - Clear the storage system
- `sizeBytes(): number` - Get the size of the storage in bytes
- `readOffset(offset: number, length: number): Uint8Array` - Read data from storage
- `writeOffset(offset: number, data: Uint8Array): void` - Write data to storage

#### `LedgerOperations`

Provides methods for working with ledger data.

- `getCursorLocalAsString(): string` - Get the local cursor as a string
- `getBlockAsJson(blockOffset: bigint): string` - Get a ledger block as JSON string
- `getValue(label: string, key: Uint8Array): Uint8Array | null` - Get a value from the ledger
- `setValue(label: string, key: Uint8Array, value: Uint8Array): void` - Set a value in the ledger
- `removeValue(label: string, key: Uint8Array): void` - Remove a value from the ledger
- `getTransactions(): Promise<any>` - Get transactions from the ledger

#### `CanisterInteraction`

Provides methods for interacting with the canister.

- `configure(config: ClientConfig): void` - Configure the canister interaction
- `callQuery(methodName: string, args: any): Promise<any>` - Generic query function
- `callUpdate(methodName: string, args: any, identity: any): Promise<any>` - Generic update function

### Function-based API (Legacy)

#### `initialize(): Promise<string>`

Initialize the WASM module, storage system, and LedgerMap. Must be called before using other functions.

#### `ledger_storage_clear(): void`

Clear the storage system.

#### `ledger_get_value(label: string, key: Uint8Array): Uint8Array | null`

Get a value from the ledger with the specified label and key.

#### `ledger_set_value(label: string, key: Uint8Array, value: Uint8Array): void`

Set a value in the ledger with the specified label, key, and value.

#### `ledger_remove_value(label: string, key: Uint8Array): void`

Remove a value from the ledger with the specified label and key.

#### `ledger_get_block_as_json(block_offset: bigint): string`

Get a ledger block as JSON string at the specified offset.

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

There's also a class-based API demo available:

```bash
cd demo
npm install
node class-based-api-demo.js
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.
