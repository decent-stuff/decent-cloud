# IC Canister Deployment Guide

This document provides clear instructions for developing and deploying the IC canister both locally and on the Internet Computer mainnet.

## Local Development

To develop locally:

1. **Start the Replica:**
   Begin by launching the local replica in the background.
   ```bash
   dfx start --background
   ```
2. **Deploy Canisters:**
   Deploy your canisters and generate the candid interfaces.
   ```bash
   dfx deploy
   ```
3. **Reset Local Environment:**
   For a fresh start, reset the state:
   ```bash
   dfx start --background --clean
   dfx deploy
   ```

### Development Tasks

The IC canister code is **completely independent** from the API server and does not use PostgreSQL or any other database. It runs entirely on the Internet Computer blockchain.

#### Running Clippy

To run clippy on the canister code (checks wasm32 compilation):
```bash
# From project root (uses makers)
makers clippy-canister

# Or directly from ic-canister directory
cargo clippy --target=wasm32-unknown-unknown
```

**Important Notes:**
- The canister code does NOT use `sqlx` or PostgreSQL
- It compiles to `wasm32-unknown-unknown` for the Internet Computer
- Test dependencies (like `pocket-ic`) are NOT checked with `--tests` flag since they run on host architecture

#### Running Tests

Canister tests use `pocket-ic` to simulate the Internet Computer environment:
```bash
# Tests run on host architecture, not wasm32
cargo test --package decent_cloud_canister
```

These tests are also checked by the regular `makers clippy` task.

## Mainnet Deployment

For production deployments on the Internet Computer mainnet:

1. **Set Up Mainnet Identity:**
   Create and switch to a mainnet identity.
   ```bash
   dfx identity new mainnet-eu
   dfx identity use mainnet-eu
   dfx identity get-principal
   ```
2. **Prepare for Deployment:**
   Check available subnet types and create a canister with an initial funding amount.
   ```bash
   dfx ledger --network ic show-subnet-types
   dfx ledger --network ic create-canister --amount 0.5 --subnet-type european <your-canister-id>
   ```
3. **Deploy Wallet and Canisters:**
   Deploy your wallet and then the canisters.
   ```bash
   dfx identity --network ic deploy-wallet <your-wallet-id>
   dfx deploy --ic
   ```
4. **Interact with Your Canister:**
   Optionally, call a method to verify deployment.
   ```bash
   dfx canister --ic call <canister-id> get_logs_info
   ```

## Local Instance Testing

After deployment, you can test your local instance:
```bash
dfx deploy --identity default
curl http://<local-canister-id>.localhost:8000/metrics
curl http://<local-canister-id>.localhost:8000/logs
```
Access the Candid UI at:
[http://127.0.0.1:8000/?canisterId=<local-canister-id>](http://127.0.0.1:8000/?canisterId=<local-canister-id>)

## Mainnet Instance Access

To interact with a mainnet deployed canister:
```bash
dfx deploy --ic --identity mainnet-eu
curl https://<canister-id>.raw.icp0.io/metrics
curl https://<canister-id>.raw.icp0.io/logs
```
Access the mainnet Candid UI via:
[https://<your-canister-id>.raw.ic0.app/](https://<your-canister-id>.raw.ic0.app/)

## Advanced: Creating a Canister with an Alternate Identity

1. **Retrieve Principal ID:**
   For a different identity:
   ```bash
   dfx identity get-principal --identity mainnet-01
   ```
2. **Check Ledger Balance:**
   Verify your account balance.
   ```bash
   dfx ledger --network ic --identity mainnet-01 balance
   ```
3. **Create a New Identity:**
   Create a new identity if needed.
   ```bash
   dfx identity new mainnet-01
   ```
4. **Create and Fund a Canister:**
   Use the ledger to create a new canister.
   ```bash
   dfx ledger --network ic --identity mainnet-01 create-canister --amount 1 <wallet-address>
   ```
   Note: The command output confirms the new canister ID.
5. **Configure Wallet:**
   Set the wallet for this identity.
   ```bash
   dfx identity --network ic --identity mainnet-01 set-wallet <wallet-id>
   ```
6. **Deploy the Canister:**
   Create and deploy the canister.
   ```bash
   dfx canister --network ic --identity mainnet-01 create decent_cloud
   dfx deploy --network ic --identity mainnet-01 decent_cloud
   ```

## API Integration

### Basic Setup

To interact with the Decent Cloud canister programmatically:

```typescript
import { Actor, HttpAgent } from '@dfinity/agent';
import { Principal } from '@dfinity/principal';
import { idlFactory } from './declarations/decent_cloud.did.js';

// Production configuration
const defaultConfig = {
  networkUrl: 'https://icp-api.io',
  canisterId: 'ggi4a-wyaaa-aaaai-actqq-cai'
};
```

### Higher-Level Client Usage

For easier interaction, you can use the provided client library:

```typescript
import { DecentCloudClient } from '@decent-stuff/dc-client';

async function example() {
  // Initialize client
  const client = new DecentCloudClient();
  await client.initialize();

  // Handle operations with built-in error handling
  try {
    // Fetch ledger blocks
    await client.fetchBlocks();

    // Get transaction history
    const lastBlock = await client.getLastFetchedBlock();
    if (lastBlock) {
      const entries = await client.getBlockEntries(lastBlock.blockOffset);
      console.log('Block entries:', entries);
    }
  } catch (error) {
    console.error('Operation failed:', error);
  }
}
```

### Security and Best Practices

1. Transactions and sensitive operations need to be signed with appropriate cryptographic signatures
2. Use secure key management for storing and handling private keys
3. Validate all input data before sending to the canister
4. Handle errors gracefully and provide appropriate user feedback
5. Use HTTPS endpoints in production environments
6. Use BigInt for all token amounts to prevent precision loss

For more details, refer to the [Documentation Home](../docs/README.md).

**Dashboard Links (replace placeholders with actual IDs):**
- **Wallet:** [Dashboard](https://dashboard.internetcomputer.org/canister/<wallet-id>)
- **Canister Code:** [Dashboard](https://dashboard.internetcomputer.org/canister/<canister-id>)
- **Subnet:** [Dashboard](https://dashboard.internetcomputer.org/subnet/<subnet-id>)
