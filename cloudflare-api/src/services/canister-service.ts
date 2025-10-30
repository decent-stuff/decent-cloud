import { Env, ResultString } from '../types';
import { CanisterMethods } from '../types-canister';
import { DatabaseService } from '../utils';

// In-memory queue for operations to flush to canister
interface PendingOperation {
  type: 'register' | 'update_profile' | 'update_offering' | 'sign_request' | 'sign_reply' | 'check_in' | 'register_user';
  pubkey: Uint8Array;
  data: Uint8Array;
  signature?: Uint8Array;
  timestamp: number;
  attempts: number;
}

export class CanisterService {
  private db: DatabaseService;
  private pendingOperations: PendingOperation[] = [];
  private lastFlushTime = 0;
  private readonly FLUSH_INTERVAL: number;
  private readonly MAX_RETRIES: number;
  private readonly CANISTER_ID: string;

  constructor(private env: Env) {
    this.db = new DatabaseService(env);
    this.FLUSH_INTERVAL = parseInt(env.FLUSH_INTERVAL_SECONDS || '5') * 1000;
    this.MAX_RETRIES = parseInt(env.MAX_RETRY_ATTEMPTS || '3');
    this.CANISTER_ID = env.CANISTER_ID;
  }

  /**
   * Handle canister-compatible method calls
   */
  async callMethod(method: keyof CanisterMethods, args: any[]): Promise<any> {
    // Try to get from D1 cache first
    const cachedResult = await this.tryGetFromCache(method, args);
    if (cachedResult !== null) {
      return cachedResult;
    }

    // Fall back to canister if not in cache or cache miss
    return await this.callCanister(method, args);
  }

  /**
   * Try to get result from D1 cache
   */
  private async tryGetFromCache(method: keyof CanisterMethods, args: any[]): Promise<any> {
    try {
      switch (method) {
        case 'provider_get_profile_by_pubkey_bytes':
          return await this.getCachedProfile(args[0]);

        case 'provider_get_profile_by_principal':
          return await this.getCachedProfileByPrincipal(args[0]);

        case 'get_identity_reputation':
          return await this.getCachedReputation(args[0]);

        case 'offering_search':
          return await this.getCachedOfferings(args[0]);

        case 'contracts_list_pending':
          return await this.getCachedContracts(args[0]);

        default:
          return null;
      }
    } catch (error) {
      console.warn(`Cache lookup failed for ${method}:`, error);
      return null;
    }
  }

  /**
   * Get cached profile from D1
   */
  private async getCachedProfile(pubkey: Uint8Array): Promise<string | null> {
    const pubkeyHex = this.uint8ArrayToHex(pubkey);
    const profile = await this.db.getProviderProfile(pubkeyHex);

    if (profile && profile.profileData) {
      // Return serialized profile data
      return new TextDecoder().decode(profile.profileData);
    }

    return null;
  }

  /**
   * Get cached profile by principal
   */
  private async getCachedProfileByPrincipal(principal: string): Promise<string | null> {
    // For now, this is not cached - would need principal->pubkey mapping
    return null;
  }

  /**
   * Get cached reputation from D1
   */
  private async getCachedReputation(pubkey: Uint8Array): Promise<bigint | null> {
    const pubkeyHex = this.uint8ArrayToHex(pubkey);
    const user = await this.db.getDcUser(pubkeyHex);

    if (user) {
      return BigInt(user.reputation);
    }

    return null;
  }

  /**
   * Get cached offerings from D1
   */
  private async getCachedOfferings(searchQuery: string): Promise<any[]> {
    // Simple implementation - in real system would parse search query
    const offerings = await this.db.getProviderOfferings();

    return offerings.map(offering => ({
      provider_pub_key: this.hexToUint8Array(offering.providerPubkey),
      offering_compressed: offering.offeringData
    }));
  }

  /**
   * Get cached contracts from D1
   */
  private async getCachedContracts(pubkey: Uint8Array | null): Promise<any[]> {
    // For now, return empty - would implement contract caching later
    return [];
  }

  /**
   * Call actual canister with error handling and retries
   */
  private async callCanister(method: keyof CanisterMethods, args: any[]): Promise<any> {
    // Use the ICP HTTP endpoint to call the canister
    const canisterUrl = `https://icp-api.io/icp/api/v2/canister/${this.CANISTER_ID}/call`;

    try {
      const response = await fetch(canisterUrl, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/cbor',
        },
        body: JSON.stringify({
          method: method.toString(),
          args: args
        })
      });

      if (!response.ok) {
        throw new Error(`Canister HTTP error: ${response.status}`);
      }

      const result = await response.json();
      return result;
    } catch (error) {
      console.warn(`Canister call failed for ${method}:`, error);

      // Return error to trigger fallback
      return { Err: `Canister call failed: ${error instanceof Error ? error.message : 'Unknown error'}` };
    }
  }

  /**
   * Queue operation for periodic flush
   */
  private async queueOperation(type: PendingOperation['type'], pubkey: Uint8Array, data: Uint8Array, signature?: Uint8Array): Promise<ResultString> {
    const operation: PendingOperation = {
      type,
      pubkey,
      data,
      signature: signature || new Uint8Array(),
      timestamp: Date.now(),
      attempts: 0
    };

    this.pendingOperations.push(operation);

    // Return success immediately - actual operation happens during flush
    return { Ok: 'Operation queued for flush' };
  }

  /**
   * Periodic flush to canister
   */
  private async flushToCanister(): Promise<void> {
    const now = Date.now();
    if (now - this.lastFlushTime < this.FLUSH_INTERVAL || this.pendingOperations.length === 0) {
      return;
    }

    console.log(`Flushing ${this.pendingOperations.length} operations to canister`);

    const operationsToFlush = [...this.pendingOperations];
    this.pendingOperations = [];

    for (const operation of operationsToFlush) {
      try {
        await this.flushOperation(operation);
      } catch (error) {
        console.error(`Failed to flush operation ${operation.type}:`, error);

        // Retry with exponential backoff
        if (operation.attempts < this.MAX_RETRIES) {
          operation.attempts++;
          const delay = Math.pow(2, operation.attempts) * 1000;
          setTimeout(() => {
            this.pendingOperations.push(operation);
          }, delay);
        }
      }
    }

    this.lastFlushTime = now;
  }

  /**
   * Flush single operation to canister
   */
  private async flushOperation(operation: PendingOperation): Promise<void> {
    console.log(`Flushing operation: ${operation.type}`, {
      pubkey: this.uint8ArrayToHex(operation.pubkey),
      dataSize: operation.data.length
    });

    // Map operation to anonymous canister method
    const methodName = this.getCanisterMethodName(operation.type);
    if (!methodName) {
      throw new Error(`Unknown operation type: ${operation.type}`);
    }

    try {
      // Prepare arguments based on operation type
      const args = this.prepareCanisterArgs(operation);

      // Call the canister via HTTP endpoint
      const canisterUrl = `https://icp-api.io/icp/api/v2/canister/${this.CANISTER_ID}/call`;

      const response = await fetch(canisterUrl, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/cbor',
        },
        body: JSON.stringify({
          method: methodName,
          args: args
        })
      });

      if (!response.ok) {
        throw new Error(`Canister HTTP error: ${response.status} ${response.statusText}`);
      }

      const result = await response.json();

      // Check if canister call was successful
      if (result && typeof result === 'object' && 'Err' in result) {
        throw new Error(`Canister operation failed: ${result.Err}`);
      }

      console.log(`Successfully flushed ${operation.type} to canister`);

    } catch (error) {
      console.error(`Failed to flush ${operation.type} to canister:`, error);
      throw error;
    }
  }

  /**
   * Get canister method name for operation type
   */
  private getCanisterMethodName(operationType: PendingOperation['type']): string | null {
    const methodMap = {
      'register': 'provider_register_anonymous',
      'update_profile': 'provider_update_profile_anonymous',
      'update_offering': 'provider_update_offering_anonymous',
      'sign_request': 'contract_sign_request_anonymous',
      'sign_reply': 'contract_sign_reply_anonymous',
      'check_in': 'provider_check_in_anonymous',
      'register_user': 'user_register_anonymous'
    };

    return methodMap[operationType] || null;
  }

  /**
   * Prepare canister arguments based on operation type
   */
  private prepareCanisterArgs(operation: PendingOperation): any[] {
    switch (operation.type) {
      case 'register':
      case 'register_user':
        return [Array.from(operation.pubkey), Array.from(operation.signature), null];

      case 'update_profile':
        return [Array.from(operation.pubkey), Array.from(operation.data), Array.from(operation.signature), null];

      case 'update_offering':
        return [Array.from(operation.pubkey), Array.from(operation.data), Array.from(operation.signature), null];

      case 'sign_request':
        return [Array.from(operation.pubkey), Array.from(operation.data), Array.from(operation.signature), null];

      case 'sign_reply':
        return [Array.from(operation.pubkey), Array.from(operation.data), Array.from(operation.signature), null];

      case 'check_in':
        const memo = new TextDecoder().decode(operation.data);
        return [Array.from(operation.pubkey), memo, Array.from(operation.signature), null];

      default:
        throw new Error(`Cannot prepare args for unknown operation type: ${operation.type}`);
    }
  }

  // Utility methods
  private uint8ArrayToHex(arr: Uint8Array): string {
    return Array.from(arr)
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');
  }

  private hexToUint8Array(hex: string): Uint8Array {
    const matches = hex.match(/.{1,2}/g);
    if (!matches) return new Uint8Array();

    return new Uint8Array(
      matches.map(byte => parseInt(byte, 16))
    );
  }

  /**
   * Start periodic flush (call this on worker startup)
   */
  startPeriodicFlush(): void {
    setInterval(() => {
      this.flushToCanister().catch(error => {
        console.error('Periodic flush failed:', error);
      });
    }, this.FLUSH_INTERVAL);
  }

  /**
   * Canister method implementations
   */

  // Provider operations
  async provider_register(pubkey_bytes: Uint8Array, crypto_sig: Uint8Array): Promise<ResultString> {
    // Queue for flush and cache locally
    await this.queueOperation('register', pubkey_bytes, crypto_sig);

    // Create user in D1 cache
    const pubkeyHex = this.uint8ArrayToHex(pubkey_bytes);
    await this.db.createDcUser({
      pubkey: pubkeyHex,
      principal: undefined,
      reputation: 0,
      balanceTokens: 0
    });

    return { Ok: 'Provider registration queued' };
  }

  async provider_update_profile(pubkey_bytes: Uint8Array, profile_serialized: Uint8Array, crypto_sig: Uint8Array): Promise<ResultString> {
    // Queue for flush and cache locally
    await this.queueOperation('update_profile', pubkey_bytes, profile_serialized, crypto_sig);

    // Update in D1 cache
    const pubkeyHex = this.uint8ArrayToHex(pubkey_bytes);
    await this.db.createProviderProfile({
      pubkey: pubkeyHex,
      profileData: profile_serialized,
      signature: crypto_sig,
      version: 1
    });

    return { Ok: 'Provider profile update queued' };
  }

  async provider_update_offering(pubkey_bytes: Uint8Array, offering_serialized: Uint8Array, crypto_sig: Uint8Array): Promise<ResultString> {
    // Generate offering ID from content hash
    const encoder = new TextEncoder();
    const content = encoder.encode(new Date().toISOString() + this.uint8ArrayToHex(offering_serialized));
    const offeringId = await crypto.subtle.digest('SHA-256', content);
    const offeringIdHex = Array.from(new Uint8Array(offeringId))
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');

    // Queue for flush and cache locally
    await this.queueOperation('update_offering', pubkey_bytes, offering_serialized, crypto_sig);

    // Update in D1 cache
    const pubkeyHex = this.uint8ArrayToHex(pubkey_bytes);
    await this.db.createProviderOffering({
      id: offeringIdHex,
      providerPubkey: pubkeyHex,
      offeringData: offering_serialized,
      signature: crypto_sig,
      version: 1,
      isActive: true
    });

    return { Ok: 'Provider offering update queued' };
  }

  async provider_list_checked_in(): Promise<ResultString> {
    // This would query cached provider list
    return { Ok: 'Checked-in provider list' };
  }

  async provider_get_profile_by_pubkey_bytes(pubkey_bytes: Uint8Array): Promise<string | null> {
    return await this.getCachedProfile(pubkey_bytes);
  }

  async provider_get_profile_by_principal(principal: string): Promise<string | null> {
    return await this.getCachedProfileByPrincipal(principal);
  }

  async offering_search(search_query: string): Promise<any[]> {
    return await this.getCachedOfferings(search_query);
  }

  // Contract operations
  async contract_sign_request(pubkey_bytes: Uint8Array, contract_info_serialized: Uint8Array, crypto_sig: Uint8Array): Promise<ResultString> {
    await this.queueOperation('sign_request', pubkey_bytes, contract_info_serialized, crypto_sig);
    return { Ok: 'Contract sign request queued' };
  }

  async contracts_list_pending(pubkey_bytes: Uint8Array | null): Promise<any[]> {
    return await this.getCachedContracts(pubkey_bytes);
  }

  async contract_sign_reply(pubkey_bytes: Uint8Array, contract_reply_serialized: Uint8Array, crypto_sig: Uint8Array): Promise<ResultString> {
    await this.queueOperation('sign_reply', pubkey_bytes, contract_reply_serialized, crypto_sig);
    return { Ok: 'Contract sign reply queued' };
  }

  // User operations
  async user_register(pubkey_bytes: Uint8Array, crypto_sig: Uint8Array): Promise<ResultString> {
    await this.queueOperation('register_user', pubkey_bytes, crypto_sig);

    // Create user in D1 cache
    const pubkeyHex = this.uint8ArrayToHex(pubkey_bytes);
    await this.db.createDcUser({
      pubkey: pubkeyHex,
      principal: undefined,
      reputation: 0,
      balanceTokens: 0
    });

    return { Ok: 'User registration queued' };
  }

  // Check-in operations
  async get_check_in_nonce(): Promise<Uint8Array> {
    // Return a nonce for check-in
    const nonce = crypto.getRandomValues(new Uint8Array(32));
    return nonce;
  }

  async provider_check_in(pubkey_bytes: Uint8Array, memo: string, nonce_crypto_sig: Uint8Array): Promise<ResultString> {
    await this.queueOperation('check_in', pubkey_bytes, new TextEncoder().encode(memo), nonce_crypto_sig);
    return { Ok: 'Provider check-in queued' };
  }

  // Common operations
  async get_identity_reputation(pubkey_bytes: Uint8Array): Promise<string> {
    const reputation = await this.getCachedReputation(pubkey_bytes);
    return reputation?.toString() || '0';
  }

  async get_registration_fee(): Promise<string> {
    // Return fixed registration fee as string for JSON serialization
    return "1000000"; // 1 DC token in smallest units
  }

  // Anonymous methods (for CF service - no caller principal)
  async provider_register_anonymous(pubkey_bytes: Uint8Array, crypto_sig: Uint8Array, _caller: null): Promise<ResultString> {
    return await this.provider_register(pubkey_bytes, crypto_sig);
  }

  async provider_update_profile_anonymous(pubkey_bytes: Uint8Array, profile_serialized: Uint8Array, crypto_sig: Uint8Array, _caller: null): Promise<ResultString> {
    return await this.provider_update_profile(pubkey_bytes, profile_serialized, crypto_sig);
  }

  async provider_update_offering_anonymous(pubkey_bytes: Uint8Array, offering_serialized: Uint8Array, crypto_sig: Uint8Array, _caller: null): Promise<ResultString> {
    return await this.provider_update_offering(pubkey_bytes, offering_serialized, crypto_sig);
  }

  async contract_sign_request_anonymous(pubkey_bytes: Uint8Array, contract_info_serialized: Uint8Array, crypto_sig: Uint8Array, _caller: null): Promise<ResultString> {
    return await this.contract_sign_request(pubkey_bytes, contract_info_serialized, crypto_sig);
  }

  async contract_sign_reply_anonymous(pubkey_bytes: Uint8Array, contract_reply_serialized: Uint8Array, crypto_sig: Uint8Array, _caller: null): Promise<ResultString> {
    return await this.contract_sign_reply(pubkey_bytes, contract_reply_serialized, crypto_sig);
  }

  async user_register_anonymous(pubkey_bytes: Uint8Array, crypto_sig: Uint8Array, _caller: null): Promise<ResultString> {
    return await this.user_register(pubkey_bytes, crypto_sig);
  }

  async provider_check_in_anonymous(pubkey_bytes: Uint8Array, memo: string, nonce_crypto_sig: Uint8Array, _caller: null): Promise<ResultString> {
    return await this.provider_check_in(pubkey_bytes, memo, nonce_crypto_sig);
  }
}