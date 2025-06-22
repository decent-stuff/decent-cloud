import init, { WasmLedgerMap } from '../../dist/wasm';

export interface LedgerMapOptions {
    labels?: string[];
}

export class LedgerMap {
    private instance: WasmLedgerMap | null = null;

    /**
     * Initialize the LedgerMap instance
     * @param options Configuration options
     */
    async initialize(options: LedgerMapOptions = {}): Promise<void> {
        await init();
        this.instance = new WasmLedgerMap(options.labels);
    }

    /**
     * Store or update a value
     * @param label The label for the entry
     * @param key The key as Uint8Array
     * @param value The value as Uint8Array
     */
    upsert(label: string, key: Uint8Array, value: Uint8Array): void {
        if (!this.instance) {
            throw new Error('LedgerMap not initialized. Call initialize() first.');
        }
        this.instance.upsert(label, key, value);
    }

    /**
     * Retrieve a value
     * @param label The label for the entry
     * @param key The key as Uint8Array
     * @returns The value as Uint8Array
     */
    get(label: string, key: Uint8Array): Uint8Array {
        if (!this.instance) {
            throw new Error('LedgerMap not initialized. Call initialize() first.');
        }
        return this.instance.get(label, key);
    }

    /**
     * Delete an entry
     * @param label The label for the entry
     * @param key The key as Uint8Array
     */
    delete(label: string, key: Uint8Array): void {
        if (!this.instance) {
            throw new Error('LedgerMap not initialized. Call initialize() first.');
        }
        this.instance.delete(label, key);
    }

    /**
     * Commit the current block of operations
     */
    commitBlock(): void {
        if (!this.instance) {
            throw new Error('LedgerMap not initialized. Call initialize() first.');
        }
        this.instance.commit_block();
    }

    /**
     * Get the total number of blocks
     */
    getBlocksCount(): number {
        if (!this.instance) {
            throw new Error('LedgerMap not initialized. Call initialize() first.');
        }
        return this.instance.get_blocks_count();
    }

    /**
     * Get the hash of the latest block
     */
    getLatestBlockHash(): Uint8Array {
        if (!this.instance) {
            throw new Error('LedgerMap not initialized. Call initialize() first.');
        }
        return this.instance.get_latest_block_hash();
    }

    /**
     * Reload the ledger from storage
     */
    refreshLedger(): void {
        if (!this.instance) {
            throw new Error('LedgerMap not initialized. Call initialize() first.');
        }
        this.instance.refresh();
    }
}

export default LedgerMap;
