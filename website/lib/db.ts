import Dexie, { Table } from 'dexie';

// Define the interface for the ledger entry
export interface LedgerEntry {
    label: string;
    key: string;
    value: unknown; // Using 'unknown' instead of 'any' for better type safety
    description: string;
    timestamp_ns?: number;
    blockVersion: number,
    blockSize: number,
    parentBlockHash: string,
    blockOffset: number,
}

// Create a Dexie database class
class LedgerDatabase extends Dexie {
    ledgerEntries!: Table<LedgerEntry, string>; // 'string' is the type of the primary key

    constructor() {
        super('DecentCloudLedgerDB');

        // Define the schema for the database
        this.version(1).stores({
            ledgerEntries: 'key' // Primary key is 'key'
        });
    }

    // Method to add or update a ledger entry
    async addOrUpdateEntry(entry: LedgerEntry): Promise<string> {
        // Add timestamp if not provided
        if (!entry.timestamp_ns) {
            entry.timestamp_ns = Date.now();
        }

        // Put will add if the key doesn't exist, or update if it does
        return await this.ledgerEntries.put(entry);
    }

    // Method to add or update multiple entries at once
    async bulkAddOrUpdate(entries: LedgerEntry[]): Promise<void> {
        await this.ledgerEntries.bulkPut(entries);
    }

    // Method to get all ledger entries
    async getAllEntries(): Promise<LedgerEntry[]> {
        return await this.ledgerEntries.toArray();
    }

    // Method to get a specific entry by key
    async getEntry(key: string): Promise<LedgerEntry | undefined> {
        return await this.ledgerEntries.get(key);
    }

    // Method to delete an entry
    async deleteEntry(key: string): Promise<void> {
        await this.ledgerEntries.delete(key);
    }

    // Method to clear all entries
    async clearAllEntries(): Promise<void> {
        await this.ledgerEntries.clear();
    }
}

// Create and export a singleton instance of the database
export const db = new LedgerDatabase();
