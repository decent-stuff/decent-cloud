/**
 * Ledger entry data provided by the WASM module.
 */
export interface RawJsonLedgerEntry {
    label: string;
    key: unknown;
    value: unknown;
    description: string;
    index?: number;
}

/**
 * Ledger block header structure.
 */
export interface RawJsonLedgerBlockHeader {
    block_version: number;
    jump_bytes_prev: number;
    jump_bytes_next: number;
    parent_block_hash: string;
    block_hash: string;
    last_bytes: string;
    offset: number;
    timestamp_ns: number;
}

/**
 * Ledger block data structure.
 */
export interface RawJsonLedgerBlock {
    block_header: RawJsonLedgerBlockHeader;
    block: RawJsonLedgerEntry[];
}

/**
 * Initialize the WASM module.
 */
export function initializeWasm(): Promise<void>;

/**
 * Parse ledger blocks from raw binary input data.
 * @param {Uint8Array} inputData - The raw input data.
 * @param {bigint} [startOffset=0n] - The starting offset.
 * @returns {Promise<RawJsonLedgerBlock[]>} A promise that resolves to an array of ledger block data.
 */
export function parseLedgerBlocks(inputData, startOffset): Promise<void>;

/**
 * Clear the ledger storage.
 */
export function ledgerStorageClear(): Promise<void>;

/**
 * Client class for interacting with the Decent Cloud Ledger Canister.
 * This class provides methods for initializing the WASM module, parsing
 * ledger blocks and converting them to JSON for showing in the UI, and
 * for clearing the ledger storage.
 */
export class DecentCloudClient {
    /**
     * Initialize the WASM module.
     */
    initialize(): Promise<void>;

    /**
     * Retrieve ledger blocks.
     * @param inputData - The raw input data.
     * @param startOffset - The starting offset.
     */
    getBlocks(inputData: Uint8Array, startOffset?: bigint): Promise<RawJsonLedgerBlock[]>;

    /**
     * Clear the ledger storage.
     */
    clearStorage(): Promise<void>;
}

import { LedgerBlock, LedgerEntry } from './db';
export { LedgerBlock, LedgerEntry };

/**
 * Ledger class for managing and interacting with the
 * in-browser ledger data snapshot stored as JSON in the local database.
 */
export class DecentCloudLedger {
    /**
     * Initialize the ledger interface.
     */
    init(): Promise<void>;

    /**
     * Fetch new ledger blocks from the remote ledger canister.
     * @returns {Promise<string>} Fetch result message.
     */
    fetchLedgerBlocks(): Promise<string>;

    /**
     * Retrieve all ledger entries stored in the local database.
     * @returns {Promise<LedgerEntry[]>} An array of all ledger entries.
     */
    getAllEntries(): Promise<LedgerEntry[]>;

    // Get all blocks from the ledger
    getAllBlocks(): Promise<LedgerBlock[]>;

    /**
     * Check if a particular provider principal is registered as a provider
     * @param principal - The string principal of the provider to check.
     * @returns {Promise<boolean>}
     */
    isProviderRegistered(principal: string): Promise<boolean>;

    /**
     * Retrieve entries for a specific block.
     * @param blockOffset The offset of the block to retrieve entries for.
     * @returns {Promise<LedgerEntry[]>} An array of ledger entries for the specified block.
     */
    getBlockEntries(blockOffset: number): Promise<LedgerEntry[]>;

    /**
     * Retrieve the last fetched ledger block entry.
     * @returns {Promise<LedgerBlock | null>} The last ledger block or null if none exists.
     */
    getLastFetchedBlock(): Promise<LedgerBlock | null>;

    /**
     * Clear the ledger storage.
     */
    clearStorage(): Promise<void>;
}

/**
 * Singleton instance of the Ledger class.
 */
export const decentCloudLedger: DecentCloudLedger;


/**
 * Function to sign data using ed25519 in a way compatible with the Decent Cloud Ledger.
 * @param {Uint8Array} secretKeyRaw - The private key, in raw format.
 * @param {Uint8Array} data - The data to sign.
 * @returns {Promise<Uint8Array>} The signature.
 */
export function ed25519Sign(secretKeyRaw: Uint8Array, data: Uint8Array): Promise<Uint8Array>;
