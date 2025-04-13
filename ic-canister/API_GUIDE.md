# Decent Cloud Canister API Guide

This guide explains how to interact with the Decent Cloud canister endpoints using the `@dfinity/agent` package.

## Setup and Error Handling

### Basic Setup

```typescript
import { Actor, HttpAgent } from '@dfinity/agent';
import { Principal } from '@dfinity/principal';
import { idlFactory } from './declarations/decent_cloud.did.js';

// Production configuration
const defaultConfig = {
  networkUrl: 'https://icp-api.io',
  canisterId: 'ggi4a-wyaaa-aaaai-actqq-cai'
};

// Initialize agent with error handling
async function createAgent(identity?: Identity) {
  try {
    const agent = new HttpAgent({
      host: defaultConfig.networkUrl,
      identity
    });
    
    await agent.fetchRootKey(); // Required when talking to local replica
    return agent;
  } catch (error) {
    console.error('Agent creation failed:', error);
    throw error;
  }
}

// Create actor with error handling
async function createActor(agent: HttpAgent) {
  try {
    return Actor.createActor(idlFactory, {
      agent,
      canisterId: defaultConfig.canisterId
    });
  } catch (error) {
    console.error('Actor creation failed:', error);
    throw error;
  }
}
```

### Robust Error Handling

```typescript
// Generic canister call wrapper
async function callCanister<T>(
  methodName: string,
  args: unknown[],
  actor: Actor
): Promise<T> {
  try {
    if (typeof actor[methodName] !== 'function') {
      throw new Error(`Method ${methodName} not found`);
    }
    
    const result = await actor[methodName](...args);
    
    // Handle variant results (Ok/Err pattern)
    if (result && typeof result === 'object' && ('Ok' in result || 'Err' in result)) {
      if ('Err' in result) {
        throw new Error(`Canister error: ${JSON.stringify(result.Err)}`);
      }
      return result.Ok as T;
    }
    
    return result as T;
  } catch (error) {
    console.error(`Error in ${methodName}:`, error);
    throw error;
  }
}
```

## Endpoints by Category

### Node Provider Operations

```typescript
// Types
interface ResultString {
  Ok: string;
  Err: string;
}

interface NodeProviderProfile {
  name: string;
  description: string;
  website?: string;
  contact: string;
  regions: string[];
  capabilities: {
    storage: string[];
    compute: string[];
    network: string[];
  };
  certifications?: string[];
}

interface NodeProviderProfileWithReputation {
  profile: NodeProviderProfile;
  reputation: bigint;
}

interface OfferingEntry {
  np_pub_key: Uint8Array;
  offering_compressed: Uint8Array;
}

interface OfferingDetails {
  id: string;
  name: string;
  description: string;
  region: string;
  specs: {
    storage: string;
    compute: string;
    network: string;
  };
  pricing: {
    model: string;    // 'on_demand' | 'reserved'
    period: string;   // 'hour' | 'day' | 'month' | 'year'
    amount_e9s: bigint;
  }[];
}

class NodeProviderManager {
  constructor(private actor: Actor) {}

  // Registration and Identity
  async register(identity: DccIdentity): Promise<ResultString> {
    const nonce = await this.getCheckInNonce();
    const signature = await identity.sign(nonce);
    return await callCanister(
      'node_provider_register',
      [identity.pubkey, signature],
      this.actor
    );
  }

  async checkIn(
    identity: DccIdentity,
    memo: string
  ): Promise<ResultString> {
    const nonce = await this.getCheckInNonce();
    const signature = await identity.sign(nonce);
    return await callCanister(
      'node_provider_check_in',
      [identity.pubkey, memo, signature],
      this.actor
    );
  }

  // Profile Management
  async updateProfile(
    identity: DccIdentity,
    profile: NodeProviderProfile
  ): Promise<ResultString> {
    if (JSON.stringify(profile).length > MAX_NP_PROFILE_BYTES) {
      throw new Error('Profile payload too large');
    }

    const serialized = this.serializeProfile(profile);
    const signature = await identity.sign(serialized);
    const fee = await this.getProfileUpdateFee();

    return await callCanister(
      'node_provider_update_profile',
      [identity.pubkey, serialized, signature],
      this.actor
    );
  }

  // Offering Management
  async updateOffering(
    identity: DccIdentity,
    offering: OfferingDetails
  ): Promise<ResultString> {
    const serialized = this.serializeOffering(offering);
    if (serialized.length > MAX_NP_OFFERING_BYTES) {
      throw new Error('Offering payload too large');
    }

    const signature = await identity.sign(serialized);
    const fee = await this.getOfferingUpdateFee();

    return await callCanister(
      'node_provider_update_offering',
      [identity.pubkey, serialized, signature],
      this.actor
    );
  }

  // Queries
  async listCheckedIn(): Promise<string[]> {
    const result = await callCanister('node_provider_list_checked_in', [], this.actor);
    return result.Ok ? JSON.parse(result.Ok) : [];
  }

  async getProfile(pubkeyBytes: Uint8Array): Promise<NodeProviderProfileWithReputation | null> {
    return await callCanister(
      'node_provider_get_profile_by_pubkey_bytes',
      [pubkeyBytes],
      this.actor
    );
  }

  async searchOfferings(query: string): Promise<OfferingEntry[]> {
    return await callCanister('offering_search', [query], this.actor);
  }

  private async getCheckInNonce(): Promise<Uint8Array> {
    return await callCanister('get_check_in_nonce', [], this.actor);
  }

  private async getProfileUpdateFee(): Promise<bigint> {
    const rewardPerBlock = await callCanister('get_registration_fee', [], this.actor);
    return rewardPerBlock / BigInt(1000);
  }

  private async getOfferingUpdateFee(): Promise<bigint> {
    const rewardPerBlock = await callCanister('get_registration_fee', [], this.actor);
    return rewardPerBlock / BigInt(10000);
  }

  private serializeProfile(profile: NodeProviderProfile): Uint8Array {
    return new TextEncoder().encode(JSON.stringify(profile));
  }

  private serializeOffering(offering: OfferingDetails): Uint8Array {
    return new TextEncoder().encode(JSON.stringify(offering));
  }
}

// Example Usage
async function nodeProviderExample() {
  const manager = new NodeProviderManager(actor);

  // Registration
  try {
    const result = await manager.register(identity);
    console.log('Registration successful:', result.Ok);

    // Update profile
    await manager.updateProfile(identity, {
      name: "Example Cloud Provider",
      description: "Enterprise storage and compute solutions",
      website: "https://example.com",
      contact: "provider@example.com",
      regions: ["us-west", "eu-central"],
      capabilities: {
        storage: ["hdd", "ssd", "nvme"],
        compute: ["cpu", "gpu"],
        network: ["1gbps", "10gbps"]
      },
      certifications: ["ISO27001", "SOC2"]
    });

    // Add offering
    await manager.updateOffering(identity, {
      id: "premium-storage",
      name: "Premium Storage Solution",
      description: "High-performance NVMe storage",
      region: "us-west",
      specs: {
        storage: "1TB NVMe",
        compute: "4 vCPU",
        network: "10Gbps"
      },
      pricing: [{
        model: "reserved",
        period: "month",
        amount_e9s: BigInt(1000000)
      }]
    });

    // Periodic check-in
    await manager.checkIn(identity, "Regular status update");

    // Query operations
    const providers = await manager.listCheckedIn();
    console.log('Active providers:', providers);

    const offerings = await manager.searchOfferings("storage");
    for (const offering of offerings) {
      const profile = await manager.getProfile(offering.np_pub_key);
      console.log('Provider:', profile?.profile.name);
      console.log('Reputation:', profile?.reputation.toString());
      console.log('Offering:', decompressOffering(offering.offering_compressed));
    }
  } catch (error) {
    if (error.message.includes('payload too large')) {
      console.error('Profile or offering exceeds size limit');
    } else {
      console.error('Operation failed:', error);
    }
  }
}
```

### Contract Management

```typescript
// Types
interface PaymentEntry {
  pricing_model: string;     // e.g., 'on_demand', 'reserved'
  time_period_unit: string;  // e.g., 'hour', 'day'
  quantity: bigint;         // number of units
  amount_e9s: bigint;      // total amount in e9s
}

interface ContractSignRequestV1 {
  requester_pubkey: Uint8Array;
  requester_ssh_pubkey: string;    // ed25519 SSH key for instance access
  requester_contact: string;       // contact information
  provider_pubkey: Uint8Array;
  offering_id: string;
  region_name?: string;
  contract_id?: string;            // for contract extensions
  instance_config?: string;        // e.g., cloud-init configuration
  payment_amount_e9s: bigint;
  payment_entries: PaymentEntry[];
  start_timestamp?: bigint;        // Unix timestamp in seconds UTC
  request_memo: string;
}

interface OpenContract {
  contract_id: string;           // hex-encoded SHA-256 of payload
  request: ContractSignRequestV1;
}

class ContractManager {
  constructor(private actor: Actor) {}

  async requestSignature(
    dccIdentity: DccIdentity,
    request: ContractSignRequestV1
  ): Promise<ResultString> {
    // Validate requester has sufficient balance (amount + fees)
    const fees = this.calculateFees(request.payment_amount_e9s);
    const requiredBalance = request.payment_amount_e9s + fees;
    
    // Serialize and sign request
    const serialized = this.serializeRequest(request);
    const signature = await dccIdentity.sign(serialized);

    return await callCanister(
      'contract_sign_request',
      [request.requester_pubkey, serialized, signature],
      this.actor
    );
  }

  async listPendingContracts(pubkeyBytes?: Uint8Array): Promise<OpenContract[]> {
    return await callCanister(
      'contracts_list_pending',
      [pubkeyBytes ? [pubkeyBytes] : []],
      this.actor
    );
  }

  async replyToContract(
    providerIdentity: DccIdentity,
    contractId: string,
    accepted: boolean,
    details: {
      instanceId?: string;
      deploymentDetails?: string;
      rejectionReason?: string;
    }
  ): Promise<ResultString> {
    const replyData = {
      contract_id: contractId,
      accepted,
      ...details
    };
    const serialized = this.serializeReply(replyData);
    const signature = await providerIdentity.sign(serialized);

    return await callCanister(
      'contract_sign_reply',
      [providerIdentity.pubkey, serialized, signature],
      this.actor
    );
  }

  private calculateFees(amount: bigint): bigint {
    return amount / BigInt(100); // 1% fee
  }
}

// Example Usage
async function contractExample() {
  const manager = new ContractManager(actor);

  // Create contract request
  const request: ContractSignRequestV1 = {
    requester_pubkey: identity.pubkey,
    requester_ssh_pubkey: 'ssh-ed25519 AAAA...', // Your ed25519 SSH public key
    requester_contact: 'email@example.com',
    provider_pubkey: providerPubkey,
    offering_id: 'storage-100tb',
    region_name: 'us-west',
    instance_config: '#cloud-config\npackages:\n  - docker\n',
    payment_amount_e9s: BigInt(1000000),
    payment_entries: [{
      pricing_model: 'reserved',
      time_period_unit: 'month',
      quantity: BigInt(12),
      amount_e9s: BigInt(1000000)
    }],
    start_timestamp: BigInt(Date.now() / 1000),
    request_memo: 'Annual storage contract'
  };

  // Submit contract request
  try {
    const result = await manager.requestSignature(identity, request);
    console.log('Contract submitted:', result.Ok);
    
    // Monitor pending contracts
    const pending = await manager.listPendingContracts(providerPubkey);
    for (const contract of pending) {
      console.log('Pending contract:', {
        id: contract.contract_id,
        offering: contract.request.offering_id,
        amount: contract.request.payment_amount_e9s.toString()
      });
    }
  } catch (error) {
    if (error.message.includes('insufficient balance')) {
      console.error('Insufficient balance for contract + fees');
    } else {
      console.error('Contract request failed:', error);
    }
  }
}
```

### Token Operations (ICRC-1, ICRC-2, ICRC-3)

```typescript
// Common types for token operations
interface Account {
  owner: Principal;
  subaccount?: Uint8Array;
}

interface TransferArgs {
  from_subaccount?: Uint8Array;
  to: Account;
  amount: bigint;
  fee?: bigint;
  memo?: Uint8Array;
  created_at_time?: bigint;
}

interface ApproveArgs {
  from_subaccount?: Uint8Array;
  spender: Account;
  amount: bigint;
  expected_allowance?: bigint;
  expires_at?: bigint;
  fee?: bigint;
  memo?: Uint8Array;
  created_at_time?: bigint;
}

// Token operations with proper error handling
class TokenOperations {
  constructor(private actor: Actor) {}

  async getMetadata() {
    return await callCanister('icrc1_metadata', [], this.actor);
  }

  async getTokenInfo() {
    const [name, symbol, decimals, fee] = await Promise.all([
      callCanister('icrc1_name', [], this.actor),
      callCanister('icrc1_symbol', [], this.actor),
      callCanister('icrc1_decimals', [], this.actor),
      callCanister('icrc1_fee', [], this.actor)
    ]);
    return { name, symbol, decimals, fee };
  }

  async transfer(args: TransferArgs) {
    return await callCanister('icrc1_transfer', [args], this.actor);
  }

  async approve(args: ApproveArgs) {
    return await callCanister('icrc2_approve', [args], this.actor);
  }

  async getTransactionHistory(start: number, length: number) {
    return await callCanister('get_transactions', [{ start, length }], this.actor);
  }
}
```

2. Balance and Transfer Operations:
```typescript
// Check balance
const getBalance = async (account: Account) => {
  return await actor.icrc1_balance_of(account);
};

// Transfer tokens
const transfer = async (args: Icrc1TransferArgs) => {
  return await actor.icrc1_transfer(args);
};

// Approve spending
const approve = async (args: ApproveArgs) => {
  return await actor.icrc2_approve(args);
};

// Transfer from approved account
const transferFrom = async (args: TransferFromArgs) => {
  return await actor.icrc2_transfer_from(args);
};
```

### Data Synchronization

```typescript
interface ResultData {
  Ok: [string, Uint8Array];
  Err: string;
}

class SyncManager {
  constructor(private actor: Actor) {}

  async fetchData(cursor?: string, bytesBefore?: Uint8Array): Promise<ResultData> {
    return await callCanister(
      'data_fetch',
      [[cursor], bytesBefore ? [bytesBefore] : []],
      this.actor
    );
  }

  async pushData(cursor: string, data: Uint8Array): Promise<ResultString> {
    const auth = await callCanister('data_push_auth', [], this.actor);
    if ('Err' in auth) {
      throw new Error(`Push authorization failed: ${auth.Err}`);
    }
    return await callCanister('data_push', [cursor, data], this.actor);
  }

  async getMetadata(): Promise<Array<[string, MetadataValue]>> {
    return await callCanister('metadata', [], this.actor);
  }
}

// Example Usage
async function syncExample() {
  const manager = new SyncManager(actor);
  
  // Fetch data incrementally
  let cursor = null;
  while (true) {
    const result = await manager.fetchData(cursor);
    if ('Err' in result) {
      console.error('Fetch failed:', result.Err);
      break;
    }
    
    const [nextCursor, data] = result.Ok;
    console.log(`Fetched ${data.length} bytes`);
    
    if (!nextCursor) break; // No more data
    cursor = nextCursor;
  }
  
  // Push data with authorization
  const dataToSync = new Uint8Array([/* ... */]);
  await manager.pushData('sync-1', dataToSync);
}
```

### Debug and Logging

```typescript
class DebugManager {
  constructor(private actor: Actor) {}

  async getLogs(level: 'debug' | 'info' | 'warn' | 'error'): Promise<ResultString> {
    return await callCanister(`get_logs_${level}`, [], this.actor);
  }

  async httpRequest(request: HttpRequest): Promise<HttpResponse> {
    return await callCanister('http_request', [request], this.actor);
  }
}

// Example Usage
async function debugExample() {
  const manager = new DebugManager(actor);
  
  // Get logs by level
  const debugLogs = await manager.getLogs('debug');
  console.log('Debug logs:', debugLogs.Ok);
  
  const errorLogs = await manager.getLogs('error');
  if ('Err' in errorLogs) {
    console.error('Failed to fetch error logs:', errorLogs.Err);
  }
  
  // Make HTTP request
  const response = await manager.httpRequest({
    method: 'GET',
    url: '/api/status',
    headers: [],
    body: new Uint8Array(),
    certificate_version: null
  });
  console.log('HTTP Response:', response.status_code);
}
```


### Identity Management

```typescript
class IdentityManager {
  constructor(private actor: Actor) {}

  async registerUser(pubkeyBytes: Uint8Array, cryptoSig: Uint8Array): Promise<ResultString> {
    return await callCanister('user_register', [pubkeyBytes, cryptoSig], this.actor);
  }

  async linkPrincipals(
    mainPrincipal: Principal,
    altPrincipals: Principal[]
  ): Promise<ResultString> {
    return await callCanister(
      'link_principals',
      [mainPrincipal, altPrincipals],
      this.actor
    );
  }

  async unlinkPrincipals(
    mainPrincipal: Principal,
    altPrincipals: Principal[]
  ): Promise<ResultString> {
    return await callCanister(
      'unlink_principals',
      [mainPrincipal, altPrincipals],
      this.actor
    );
  }

  async getLinkedPrincipals(mainPrincipal: Principal): Promise<Principal[]> {
    const result = await callCanister(
      'list_alt_principals',
      [mainPrincipal],
      this.actor
    );
    return result.Ok || [];
  }

  async getReputation(pubkeyBytes: Uint8Array): Promise<bigint> {
    return await callCanister(
      'get_identity_reputation',
      [pubkeyBytes],
      this.actor
    );
  }
}

// Example Usage
async function identityExample() {
  const manager = new IdentityManager(actor);
  
  // Register new user
  const signature = await signData(pubkeyBytes);
  await manager.registerUser(pubkeyBytes, signature);
  
  // Link additional principals
  const altIdentities = [
    Principal.fromText('identity-1'),
    Principal.fromText('identity-2')
  ];
  await manager.linkPrincipals(mainPrincipal, altIdentities);
  
  // Check reputation
  const reputation = await manager.getReputation(pubkeyBytes);
  console.log('Current reputation:', reputation.toString());
}
```

### Transaction History

```typescript
interface GetBlocksArgs {
  start: bigint;
  length: number;
}

interface GetTransactionsRequest {
  start: bigint;
  length: number;
}

class TransactionManager {
  constructor(private actor: Actor) {}

  async getBlocks(args: GetBlocksArgs) {
    return await callCanister('get_blocks', [args], this.actor);
  }

  async getTransactions(args: GetTransactionsRequest) {
    return await callCanister('get_transactions', [args], this.actor);
  }

  async getBlocksCertificate() {
    return await callCanister('get_data_certificate', [], this.actor);
  }

  async getBlockArchives() {
    return await callCanister('icrc3_get_archives', [{ from: null }], this.actor);
  }
}

// Example Usage
async function transactionExample() {
  const manager = new TransactionManager(actor);
  
  // Get latest transactions
  const txs = await manager.getTransactions({
    start: BigInt(0),
    length: 10
  });
  
  // Get blocks with certificate
  const blocks = await manager.getBlocks({
    start: BigInt(0),
    length: 10
  });
  const certificate = await manager.getBlocksCertificate();
  
  // Get archive information
  const archives = await manager.getBlockArchives();
  console.log('Available archives:', archives);
}
```

## Error Handling

All endpoints can potentially return errors. Make sure to handle them appropriately:

```typescript
try {
  const result = await actor.icrc1_transfer(transferArgs);
  if ('Err' in result) {
    // Handle specific transfer error
    console.error('Transfer failed:', result.Err);
  } else {
    // Success case
    console.log('Transfer successful, block height:', result.Ok);
  }
} catch (error) {
  // Handle unexpected errors
  console.error('Unexpected error:', error);
}
```

## Security and Best Practices

1. Always sign transactions and sensitive operations with appropriate cryptographic signatures
2. Use secure key management for storing and handling private keys
3. Validate all input data before sending to the canister
4. Handle errors gracefully and provide appropriate user feedback
5. Use HTTPS endpoints in production environments
6. Use BigInt for all token amounts to prevent precision loss
7. Implement proper error handling and retries for network issues
8. Keep track of transaction nonces to prevent duplicates
9. Use proper type checking for all canister responses
10. Implement rate limiting for API calls

## Higher-Level Client Usage

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

## Common Error Types

```typescript
// Token transfer errors
type TransferError =
  | { BadFee: { expected_fee: bigint } }
  | { BadBurn: { min_burn_amount: bigint } }
  | { InsufficientFunds: { balance: bigint } }
  | { TooOld: null }
  | { CreatedInFuture: { ledger_time: bigint } }
  | { TemporarilyUnavailable: null }
  | { Duplicate: { duplicate_of: bigint } };

// Approval errors
type ApproveError =
  | { BadFee: { expected_fee: bigint } }
  | { InsufficientFunds: { balance: bigint } }
  | { AllowanceChanged: { current_allowance: bigint } }
  | { Expired: { ledger_time: bigint } }
  | { TooOld: null };
```