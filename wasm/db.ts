import Dexie, { Table } from 'dexie';

/**
 * LedgerDatabase class for managing ledger data in IndexedDB
 * Uses Dexie as the IndexedDB wrapper
 */
class LedgerDatabase extends Dexie {
    ledgerBlocks!: Table<LedgerBlock, number>;
    ledgerEntries!: Table<LedgerEntry, string>;

    constructor() {
        super('DecentCloudLedgerDB');

        // Define stores in a single version declaration
        this.version(2).stores({
            ledgerBlocks: 'blockOffset, timestampNs',
            ledgerEntries: '[label+key], *blockOffset',
        });
    }

    /**
     * Get the last ledger entry (with the highest timestamp)
     * @returns The last ledger entry or null if no entries exist
     */
    async getLastBlock(): Promise<LedgerBlock | null> {
        return await this.ledgerBlocks.orderBy('timestampNs').last() || null;
    }

    /**
     * Add or update multiple ledger entries in a single transaction
     * Also updates the last entry if any of the new entries has a higher timestamp
     * @param newBlocks The ledger blocks to add
     * @param newEntries The ledger entries to add or update
     */
    async bulkAddOrUpdate(newBlocks: LedgerBlock[], newEntries: LedgerEntry[]): Promise<void> {
        if (newEntries.length === 0) return;

        await this.transaction('rw', [this.ledgerBlocks, this.ledgerEntries], async () => {
            // Add or update all blocks
            await this.ledgerBlocks.bulkPut(newBlocks);
            // Add or update all entries
            await this.ledgerEntries.bulkPut(newEntries);
        });
    }

    /**
     * Get all ledger entries
     * @returns All ledger entries
     */
    async getAllEntries(): Promise<LedgerEntry[]> {
        return await this.ledgerEntries.toArray();
    }

    /**
     * Get all ledger blocks
     * @returns All ledger blocks
     */
    async getAllBlocks(): Promise<LedgerBlock[]> {
        return await this.ledgerBlocks.toArray();
    }

    /**
     * Retrieve entries for a specific block.
     *
     * @param blockOffset The offset of the block to retrieve entries for.
     * @returns {Promise<LedgerEntry[]>} An array of ledger entries for the specified block.
     */
    async getBlockEntries(blockOffset: number): Promise<LedgerEntry[]> {
        try {
            if (typeof blockOffset !== 'number') {
                throw new Error(`blockOffset must be a number, got (${typeof blockOffset}) ${blockOffset} instead`);
            }
            return await this.ledgerEntries.where('blockOffset').equals(blockOffset).toArray();
        } catch (error) {
            console.error("Error retrieving ledger entries for block:", error);
            throw error;
        }
    }

    /**
     * Get a specific ledger entry by key
     * @param key The key of the entry to get
     * @returns The ledger entry or undefined if not found
     */
    async getEntry(key: string): Promise<LedgerEntry | undefined> {
        return await this.ledgerEntries.get(key);
    }

    /**
     * Clear all ledger entries from the database
     */
    async clearAllEntries(): Promise<void> {
        await this.transaction('rw', [this.ledgerBlocks, this.ledgerEntries], async () => {
            await this.ledgerBlocks.clear();
            await this.ledgerEntries.clear();
        });
    }
}

// Create and export a singleton instance of the database
export const db = new LedgerDatabase();

export interface LedgerBlock {
    blockVersion: number;
    blockSize: number;
    parentBlockHash: string;
    blockOffset: number;
    fetchCompareBytes: string;
    fetchOffset: number;
    timestampNs: number;
}

export interface LedgerEntry {
    label: string;
    key: string;
    value: unknown;
    description: string;
    blockOffset: number;
}
