/**
 * ICP Agent for communicating with the Decent Cloud canister
 * 
 * This service provides a real connection to the ICP canister using the
 * agent-js library and supports querying ledger data for synchronization.
 */

import { Env } from '../types';
import { Principal } from '@dfinity/principal';
import { Actor, HttpAgent } from '@dfinity/agent';
import { IDL } from '@dfinity/candid';

// Canister interface types matching the DID file
export interface NextBlockEntry {
  label: string;
  key: Uint8Array;
  value: Uint8Array;
}

export interface NextBlockEntriesResult {
  entries: NextBlockEntry[];
  has_more: boolean;
  total_count: number;
}

export interface NextBlockSyncResponse {
  has_block: boolean;
  block_header?: Uint8Array;
  block_data?: Uint8Array;
  block_hash?: Uint8Array;
  block_position?: bigint;
  next_block_position?: bigint;
  entries_count: bigint;
  more_blocks_available: boolean;
}

// IDL definitions for canister methods
const idlFactory = ({ IDL }: any) => {
  const NextBlockEntry = IDL.Record({
    'label': IDL.Text,
    'key': IDL.Vec(IDL.Nat8),
    'value': IDL.Vec(IDL.Nat8),
  });

  const NextBlockEntriesResult = IDL.Record({
    'entries': IDL.Vec(NextBlockEntry),
    'has_more': IDL.Bool,
    'total_count': IDL.Nat32,
  });

  const NextBlockSyncResponse = IDL.Record({
    'has_block': IDL.Bool,
    'block_header': IDL.Opt(IDL.Vec(IDL.Nat8)),
    'block_data': IDL.Opt(IDL.Vec(IDL.Nat8)),
    'block_hash': IDL.Opt(IDL.Vec(IDL.Nat8)),
    'block_position': IDL.Opt(IDL.Nat64),
    'next_block_position': IDL.Opt(IDL.Nat64),
    'entries_count': IDL.Nat,
    'more_blocks_available': IDL.Bool,
  });

  const ResultString = IDL.Variant({
    'Ok': IDL.Text,
    'Err': IDL.Text,
  });

  const ResultData = IDL.Variant({
    'Ok': IDL.Tuple(IDL.Text, IDL.Vec(IDL.Nat8)),
    'Err': IDL.Text,
  });

  return IDL.Service({
    'next_block_entries': IDL.Func(
      [IDL.Opt(IDL.Text), IDL.Opt(IDL.Nat32), IDL.Opt(IDL.Nat32)],
      [NextBlockEntriesResult],
      ['query']
    ),
    'provider_list_checked_in': IDL.Func(
      [],
      [ResultString],
      ['query']
    ),
    'data_fetch': IDL.Func(
      [IDL.Opt(IDL.Text), IDL.Opt(IDL.Vec(IDL.Nat8))],
      [ResultData],
      ['query']
    ),
    'next_block_sync': IDL.Func(
      [IDL.Opt(IDL.Nat64), IDL.Opt(IDL.Bool), IDL.Opt(IDL.Nat32)],
      [IDL.Variant({ 'Ok': NextBlockSyncResponse, 'Err': IDL.Text })],
      ['query']
    ),
    'provider_get_profile_by_pubkey_bytes': IDL.Func(
      [IDL.Vec(IDL.Nat8)],
      [IDL.Opt(IDL.Text)],
      ['query']
    ),
    'offering_search': IDL.Func(
      [IDL.Text],
      [IDL.Vec(IDL.Tuple(IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8)))],
      ['query']
    ),
    'get_identity_reputation': IDL.Func(
      [IDL.Vec(IDL.Nat8)],
      [IDL.Nat64],
      ['query']
    ),
    'provider_register_anonymous': IDL.Func(
      [IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8), IDL.Opt(IDL.Text)],
      [ResultString],
      []
    ),
    'provider_update_profile_anonymous': IDL.Func(
      [IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8), IDL.Opt(IDL.Text)],
      [ResultString],
      []
    ),
    'provider_update_offering_anonymous': IDL.Func(
      [IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8), IDL.Opt(IDL.Text)],
      [ResultString],
      []
    ),
  });
};

export interface IcpCanisterActor {
  next_block_entries: (
    label: [] | [string],
    offset: [] | [number],
    limit: [] | [number]
  ) => Promise<NextBlockEntriesResult>;

  provider_list_checked_in: () => Promise<{ Ok: string } | { Err: string }>;

  data_fetch: (
    cursor: [] | [string],
    bytes_before: [] | [Uint8Array]
  ) => Promise<{ Ok: [string, Uint8Array] } | { Err: string }>;

  next_block_sync: (
    start_position: [] | [bigint],
    include_data: [] | [boolean],
    max_entries: [] | [number]
  ) => Promise<{ Ok: NextBlockSyncResponse } | { Err: string }>;

  provider_get_profile_by_pubkey_bytes: (pubkey: Uint8Array) => Promise<[] | [string]>;

  offering_search: (query: string) => Promise<Array<[Uint8Array, Uint8Array]>>;

  get_identity_reputation: (pubkey: Uint8Array) => Promise<bigint>;

  provider_register_anonymous: (
    pubkey: Uint8Array,
    signature: Uint8Array,
    principal: [] | [string]
  ) => Promise<{ Ok: string } | { Err: string }>;

  provider_update_profile_anonymous: (
    pubkey: Uint8Array,
    profile: Uint8Array,
    signature: Uint8Array,
    principal: [] | [string]
  ) => Promise<{ Ok: string } | { Err: string }>;

  provider_update_offering_anonymous: (
    pubkey: Uint8Array,
    offering: Uint8Array,
    signature: Uint8Array,
    principal: [] | [string]
  ) => Promise<{ Ok: string } | { Err: string }>;
}

/**
 * ICP Agent for interacting with the Decent Cloud canister
 */
export class IcpAgent {
  private agent: HttpAgent;
  private actor: IcpCanisterActor | null = null;
  private canisterId: Principal;
  private icpHost: string;

  constructor(private env: Env) {
    this.canisterId = Principal.fromText(env.CANISTER_ID);
    // Use IC mainnet by default, or local replica for testing
    this.icpHost = env.ENVIRONMENT === 'development' 
      ? 'http://localhost:4943' 
      : 'https://icp-api.io';
    
    this.agent = new HttpAgent({
      host: this.icpHost,
    });

    // Only fetch root key in development (local replica)
    if (env.ENVIRONMENT === 'development') {
      this.agent.fetchRootKey().catch(err => {
        console.warn('Unable to fetch root key:', err);
      });
    }
  }

  /**
   * Initialize the actor if not already initialized
   */
  private async ensureActor(): Promise<IcpCanisterActor> {
    if (!this.actor) {
      this.actor = Actor.createActor<IcpCanisterActor>(idlFactory, {
        agent: this.agent,
        canisterId: this.canisterId,
      });
    }
    return this.actor;
  }

  /**
   * Query next block entries from the canister
   * @param label Optional label to filter entries (e.g., 'ProvProfile', 'ProvOffering')
   * @param offset Starting offset for pagination
   * @param limit Maximum number of entries to return
   */
  async getNextBlockEntries(
    label?: string,
    offset: number = 0,
    limit: number = 100
  ): Promise<NextBlockEntriesResult> {
    const actor = await this.ensureActor();
    return await actor.next_block_entries(
      label ? [label] : [],
      [offset],
      [limit]
    );
  }

  /**
   * Query next block sync data (alternative method)
   * @param startPosition Starting position in the ledger
   * @param includeData Whether to include full data
   * @param maxEntries Maximum entries to return
   */
  async getNextBlockSync(
    startPosition?: bigint,
    includeData: boolean = true,
    maxEntries: number = 100
  ): Promise<NextBlockSyncResponse> {
    const actor = await this.ensureActor();
    const result = await actor.next_block_sync(
      startPosition ? [startPosition] : [],
      [includeData],
      [maxEntries]
    );

    if ('Err' in result) {
      throw new Error(`Canister error: ${result.Err}`);
    }

    return result.Ok;
  }

  /**
   * Get provider profile by public key
   */
  async getProviderProfile(pubkey: Uint8Array): Promise<string | null> {
    const actor = await this.ensureActor();
    const result = await actor.provider_get_profile_by_pubkey_bytes(pubkey);
    return result.length > 0 ? (result[0] ?? null) : null;
  }

  /**
   * Search offerings
   */
  async searchOfferings(query: string): Promise<Array<{ provider_pub_key: Uint8Array; offering_compressed: Uint8Array }>> {
    const actor = await this.ensureActor();
    const results = await actor.offering_search(query);
    return results.map(([provider_pub_key, offering_compressed]) => ({
      provider_pub_key,
      offering_compressed,
    }));
  }

  /**
   * Get identity reputation
   */
  async getReputation(pubkey: Uint8Array): Promise<bigint> {
    const actor = await this.ensureActor();
    return await actor.get_identity_reputation(pubkey);
  }

  /**
   * Register provider (anonymous call)
   */
  async registerProvider(pubkey: Uint8Array, signature: Uint8Array): Promise<string> {
    const actor = await this.ensureActor();
    const result = await actor.provider_register_anonymous(pubkey, signature, []);
    
    if ('Err' in result) {
      throw new Error(`Registration failed: ${result.Err}`);
    }
    
    return result.Ok;
  }

  /**
   * Update provider profile (anonymous call)
   */
  async updateProviderProfile(
    pubkey: Uint8Array,
    profile: Uint8Array,
    signature: Uint8Array
  ): Promise<string> {
    const actor = await this.ensureActor();
    const result = await actor.provider_update_profile_anonymous(pubkey, profile, signature, []);
    
    if ('Err' in result) {
      throw new Error(`Profile update failed: ${result.Err}`);
    }
    
    return result.Ok;
  }

  /**
   * Update provider offering (anonymous call)
   */
  async updateProviderOffering(
    pubkey: Uint8Array,
    offering: Uint8Array,
    signature: Uint8Array
  ): Promise<string> {
    const actor = await this.ensureActor();
    const result = await actor.provider_update_offering_anonymous(pubkey, offering, signature, []);
    
    if ('Err' in result) {
      throw new Error(`Offering update failed: ${result.Err}`);
    }
    
    return result.Ok;
  }

  /**
   * Utility: Convert hex string to Uint8Array
   */
  static hexToUint8Array(hex: string): Uint8Array {
    const matches = hex.match(/.{1,2}/g);
    if (!matches) return new Uint8Array();
    return new Uint8Array(matches.map(byte => parseInt(byte, 16)));
  }

  /**
   * Utility: Convert Uint8Array to hex string
   */
  static uint8ArrayToHex(arr: Uint8Array): string {
    return Array.from(arr)
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');
  }
}
