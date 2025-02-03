# Decent Cloud Web Client

A browser-based client for interacting with the Decent Cloud ledger canister.

## Installation

```bash
npm install @decent-stuff/web-agent
```

## Usage

```typescript
import { DecentCloudWebClient } from "@decent-stuff/web-agent";

// Initialize the client
const client = new DecentCloudWebClient(
  "https://icp-api.io",
  "ggi4a-wyaaa-aaaai-actqq-cai"
);

// Fetch latest ledger data
await client.fetchLedgerData();

// Get current position
const position = client.getCurrentPosition();
console.log(`Current ledger position: ${position}`);

// Clear stored data
client.clearStorage();
```

## Features

- Fetch and store ledger data in browser's localStorage
- Track ledger position for incremental updates
- Efficient data storage and retrieval
- TypeScript support

## Example

Check out the [basic usage example](./examples/basic-usage.html) for a complete implementation.

## Development

```bash
# Install dependencies
npm install

# Build the package
npm run build

# Watch for changes during development
npm run dev
```

## Browser Support

The client uses the following browser APIs:

- `localStorage` for data persistence
- `Uint8Array` for binary data handling
- Modern ES6+ features

Make sure your target browsers support these features.

## License

MIT
