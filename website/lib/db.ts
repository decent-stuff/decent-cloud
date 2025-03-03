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


// Create a Dexie database class with a dedicated store for the last entry
class LedgerDatabase extends Dexie {
    ledgerEntries!: Table<LedgerEntry, string>;
    lastLedgerEntry!: Table<LedgerEntry, string>; // Store for quick access to the entry with the highest timestamp

    // Constant key used for the "last entry" in the dedicated store
    private readonly lastEntryKey = 'lastEntry';

    constructor() {
        super('DecentCloudLedgerDB');

        // Define both stores in a single version declaration
        this.version(2).stores({
            ledgerEntries: 'key, timestamp_ns, blockOffset',
            lastLedgerEntry: 'key'
        });
    }

    // Method to add or update a ledger entry
    async addOrUpdateEntry(entry: LedgerEntry): Promise<string> {
        // Assign timestamp if not provided
        if (!entry.timestamp_ns) {
            entry.timestamp_ns = Date.now();
        }

        // Put will add if the key doesn't exist, or update if it does
        await this.ledgerEntries.put(entry);

        // Retrieve current last entry from the dedicated store
        const currentLast = await this.lastLedgerEntry.get(this.lastEntryKey);
        // Update the last entry if this entry is newer
        if (
            !currentLast ||
            (entry.timestamp_ns && (!currentLast.timestamp_ns || entry.timestamp_ns > currentLast.timestamp_ns))
        ) {
            await this.lastLedgerEntry.put({ ...entry }, this.lastEntryKey);
        }
        return entry.key;
    }

    // Method to add or update multiple entries at once
    async bulkAddOrUpdate(entries: LedgerEntry[]): Promise<void> {
        await this.ledgerEntries.bulkPut(entries);
        // Determine the entry with the highest timestamp from the new entries
        const maxEntry = entries.reduce((prev, curr) =>
            (!prev || (curr.timestamp_ns && prev.timestamp_ns && curr.timestamp_ns > prev.timestamp_ns))
                ? curr
                : prev,
            null as LedgerEntry | null
        );

        if (maxEntry) {
            const currentLast = await this.lastLedgerEntry.get(this.lastEntryKey);
            if (
                !currentLast ||
                (maxEntry.timestamp_ns && (!currentLast.timestamp_ns || maxEntry.timestamp_ns > currentLast.timestamp_ns))
            ) {
                await this.lastLedgerEntry.put({ ...maxEntry }, this.lastEntryKey);
            }
        }
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
        // Check if the entry to be deleted is the current "last entry"
        const currentLast = await this.lastLedgerEntry.get(this.lastEntryKey);
        if (currentLast && currentLast.key === key) {
            await this.ledgerEntries.delete(key);
            // Recalculate the new last entry by ordering by timestamp_ns and taking the last one
            const newLast = await this.ledgerEntries.orderBy('timestamp_ns').last();
            if (newLast) {
                await this.lastLedgerEntry.put({ ...newLast }, this.lastEntryKey);
            } else {
                // No entries left: clear the lastLedgerEntry store
                await this.lastLedgerEntry.delete(this.lastEntryKey);
            }
        } else {
            await this.ledgerEntries.delete(key);
        }
    }

    // Method to clear all entries
    async clearAllEntries(): Promise<void> {
        await this.ledgerEntries.clear();
        await this.lastLedgerEntry.clear();
    }

    // Method to get the ledger entry with the highest timestamp (quick access)
    async getLastEntry(): Promise<LedgerEntry | null> {
        return (await this.lastLedgerEntry.get(this.lastEntryKey)) || null;
    }
}

// Create and export a singleton instance of the database
export const db = new LedgerDatabase();
