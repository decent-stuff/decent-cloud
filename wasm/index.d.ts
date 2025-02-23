/* tslint:disable */
/* eslint-disable */

/**
 * Initialize the WASM module, storage system, and LedgerMap
 */
export function initialize(): Promise<string>;

/**
 * Clear the storage system
 */
export function ledger_storage_clear(): void;

/**
 * Get a value from the ledger
 * @param key The key as Uint8Array
 * @returns The value as Uint8Array if found, null otherwise
 */
export function ledger_get_value(key: Uint8Array): Uint8Array | null;

/**
 * Set a value in the ledger
 * @param key The key as Uint8Array
 * @param value The value as Uint8Array
 */
export function ledger_set_value(key: Uint8Array, value: Uint8Array): void;

/**
 * Remove a value from the ledger
 * @param key The key as Uint8Array
 */
export function ledger_remove_value(key: Uint8Array): void;

/**
 * Get a ledger block as JSON
 * @param block_offset The block offset as u64
 */
export function ledger_get_block_as_json(block_offset: bigint): { Ok: string } | { Err: string };

/**
 * Generic query function that can be used for any query method
 */
export function call_query_canister(method_name: string, arg: any): Promise<any>;

/**
 * Generic update function that can be used for any update method
 */
export function call_update_canister(method_name: string, arg: any, identity: any): Promise<any>;

/**
 * Get transactions
 */
export function get_transactions(): Promise<any>;
