/* tslint:disable */
/* eslint-disable */

/**
 * Configuration options for the DecentCloudClient
 */
export interface ClientConfig {
    /**
     * The URL of the Internet Computer network
     * @default 'https://icp-api.io'
     */
    networkUrl?: string;

    /**
     * The canister ID to interact with
     * @default 'ggi4a-wyaaa-aaaai-actqq-cai'
     */
    canisterId?: string;
}

/**
 * Main client class for interacting with Decent Cloud
 */
export class DecentCloudClient {
    /**
     * Create a new DecentCloudClient instance
     * @param config Optional configuration options
     */
    constructor(config?: ClientConfig);

    /**
     * Initialize the WASM module, storage system, and LedgerMap
     * @returns A promise that resolves when initialization is complete
     */
    initialize(): Promise<string>;

    /**
     * Access to ledger storage operations
     */
    readonly storage: LedgerStorage;

    /**
     * Access to ledger data operations
     */
    readonly ledger: LedgerOperations;

    /**
     * Access to canister interaction methods
     */
    readonly canister: CanisterInteraction;
}

/**
 * Ledger storage operations
 */
export interface LedgerStorage {
    /**
     * Clear the storage system
     */
    clear(): void;

    /**
     * Get the size of the storage in bytes
     */
    sizeBytes(): number;

    /**
     * Read data from storage at a specific offset
     * @param offset The offset to read from
     * @param length The number of bytes to read
     */
    readOffset(offset: number, length: number): Uint8Array;

    /**
     * Write data to storage at a specific offset
     * @param offset The offset to write to
     * @param data The data to write
     */
    writeOffset(offset: number, data: Uint8Array): void;
}

/**
 * Ledger data operations
 */
export interface LedgerOperations {
    /**
     * Get the local cursor as a string
     */
    getCursorLocalAsString(): string;

    /**
     * Get a ledger block as JSON
     * @param blockOffset The block offset
     * @returns A JSON string representation of the block
     */
    getBlockAsJson(blockOffset: bigint): string;

    /**
     * Get a value from the ledger
     * @param label The label for the key-value pair
     * @param key The key as Uint8Array
     * @returns The value as Uint8Array if found, null otherwise
     */
    getValue(label: string, key: Uint8Array): Uint8Array | null;

    /**
     * Set a value in the ledger
     * @param label The label for the key-value pair
     * @param key The key as Uint8Array
     * @param value The value as Uint8Array
     */
    setValue(label: string, key: Uint8Array, value: Uint8Array): void;

    /**
     * Remove a value from the ledger
     * @param label The label for the key-value pair
     * @param key The key as Uint8Array
     */
    removeValue(label: string, key: Uint8Array): void;

    /**
     * Get transactions
     */
    getTransactions(): Promise<any>;
}

/**
 * Canister interaction methods
 */
export interface CanisterInteraction {
    /**
     * Generic query function that can be used for any query method
     * @param methodName The name of the method to call
     * @param args The arguments to pass to the method
     */
    callQuery(methodName: string, args: any): Promise<any>;

    /**
     * Generic update function that can be used for any update method
     * @param methodName The name of the method to call
     * @param args The arguments to pass to the method
     * @param identity The identity to use for the call
     */
    callUpdate(methodName: string, args: any, identity: any): Promise<any>;

    /**
     * Configure the canister interaction
     * @param config The configuration options
     */
    configure(config: ClientConfig): void;
}

/**
 * Create a new DecentCloudClient instance
 * @param config Optional configuration options
 */
export function createClient(config?: ClientConfig): DecentCloudClient;
