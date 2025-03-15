/**
 * Ledger entry data provided by the WASM module.
 */
export interface LedgerEntryData {
    label: string;
    key: unknown;
    value: unknown;
    description: string;
    index?: number;
}

/**
 * Ledger block header structure.
 */
export interface BlockHeader {
    block_version: number;
    jump_bytes_prev: number;
    jump_bytes_next: number;
    parent_block_hash: string;
    last_bytes: string;
    offset: number;
    timestamp_ns: number;
}

/**
 * Ledger block data structure.
 */
export interface BlockData {
    block_header: BlockHeader;
    block: LedgerEntryData[];
}

/**
 * Initialize the WASM module.
 */
export function initializeWasm(): Promise<void>;

/**
 * Parse ledger blocks from raw binary input data.
 * @param {Uint8Array} inputData - The raw input data.
 * @param {bigint} [startOffset=0n] - The starting offset.
 * @returns {Promise<BlockData[]>} A promise that resolves to an array of ledger block data.
 */
export function parseLedgerBlocks(inputData, startOffset): Promise<void>;

/**
 * Clear the ledger storage.
 */
export function ledgerStorageClear(): Promise<void>;

/**
 * Client class for interacting with the Decent Cloud ledger.
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
    getBlocks(inputData: Uint8Array, startOffset?: bigint): Promise<BlockData[]>;

    /**
     * Clear the ledger storage.
     */
    clearStorage(): Promise<void>;
}
