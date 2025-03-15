import Dexie from 'dexie';
/**
 * LedgerDatabase class for managing ledger data in IndexedDB
 * Uses Dexie as the IndexedDB wrapper
 */
class LedgerDatabase extends Dexie {
    constructor() {
        super('DecentCloudLedgerDB');
        // Constant key used for the "last entry" in the dedicated store
        this.lastEntryKey = 'lastEntry';
        // Define stores in a single version declaration
        this.version(3).stores({
            ledgerEntries: 'key, timestamp_ns, blockOffset',
            lastLedgerEntry: 'key',
        });
    }
    /**
     * Get the last ledger entry (with the highest timestamp)
     * @returns The last ledger entry or null if no entries exist
     */
    async getLastEntry() {
        return (await this.lastLedgerEntry.get(this.lastEntryKey)) || null;
    }
    /**
     * Add or update multiple ledger entries in a single transaction
     * Also updates the last entry if any of the new entries has a higher timestamp
     * @param entries The ledger entries to add or update
     */
    async bulkAddOrUpdate(entries) {
        if (entries.length === 0)
            return;
        await this.transaction('rw', [this.ledgerEntries, this.lastLedgerEntry], async () => {
            // Add or update all entries
            await this.ledgerEntries.bulkPut(entries);
            // Find the entry with the highest timestamp
            const maxEntry = entries.reduce((prev, curr) => (!prev || (curr.parentBlockHash && curr.timestamp_ns && prev.timestamp_ns && curr.timestamp_ns > prev.timestamp_ns))
                ? curr
                : prev, null);
            if (maxEntry) {
                const currentLast = await this.lastLedgerEntry.get(this.lastEntryKey);
                const new_ts = maxEntry?.timestamp_ns;
                const prev_ts = currentLast?.ledgerEntry.timestamp_ns;
                // Update the last entry if this entry has a higher timestamp
                if (new_ts !== undefined && (prev_ts === undefined || new_ts > prev_ts)) {
                    const lastEntry = {
                        ledgerEntry: { ...maxEntry },
                        bytesBefore: new Uint8Array(0) // This will be updated by the caller if needed
                    };
                    await this.lastLedgerEntry.put(lastEntry, this.lastEntryKey);
                }
            }
        });
    }
    /**
     * Update the bytesBefore field of the last entry
     * @param bytesBefore The bytes before the last entry
     */
    async updateLastEntryBytes(bytesBefore) {
        const lastEntry = await this.getLastEntry();
        if (lastEntry) {
            lastEntry.bytesBefore = bytesBefore;
            await this.lastLedgerEntry.put(lastEntry, this.lastEntryKey);
        }
    }
    /**
     * Get all ledger entries
     * @returns All ledger entries
     */
    async getAllEntries() {
        return await this.ledgerEntries.toArray();
    }
    /**
     * Get a specific ledger entry by key
     * @param key The key of the entry to get
     * @returns The ledger entry or undefined if not found
     */
    async getEntry(key) {
        return await this.ledgerEntries.get(key);
    }
    /**
     * Clear all ledger entries from the database
     */
    async clearAllEntries() {
        await this.transaction('rw', [this.ledgerEntries, this.lastLedgerEntry], async () => {
            await this.ledgerEntries.clear();
            await this.lastLedgerEntry.clear();
        });
    }
}
// Create and export a singleton instance of the database
export const db = new LedgerDatabase();
