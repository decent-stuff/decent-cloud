import Dexie, { Table } from 'dexie';
import { fetchDataWithCache } from './agent_js_wrapper.js';
import { ledger_get_block_as_json } from './client.js';

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

// Define the interface for the binary block data
export interface LedgerBinaryBlock {
    blockOffset: number;
    data: Uint8Array;
}

// Create a Dexie database class with dedicated stores
class LedgerDatabase extends Dexie {
    ledgerEntries!: Table<LedgerEntry, string>;
    lastLedgerEntry!: Table<LedgerEntry, string>; // Store for quick access to the entry with the highest timestamp
    ledgerBinaryBlocks!: Table<LedgerBinaryBlock, number>; // Store for binary block data

    // Constant key used for the "last entry" in the dedicated store
    private readonly lastEntryKey = 'lastEntry';

    constructor() {
        super('DecentCloudLedgerDB');

        // Define stores in a single version declaration
        this.version(3).stores({
            ledgerEntries: 'key, timestamp_ns, blockOffset',
            lastLedgerEntry: 'key',
            ledgerBinaryBlocks: 'blockOffset'
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

        if (entry.key !== null) {
            console.debug('addOrUpdateEntry setting max entry:', entry);
            // Update the last entry if this entry is newer
            const currentLast = await this.lastLedgerEntry.get(this.lastEntryKey);
            const new_ts = entry?.timestamp_ns;
            const prev_ts = currentLast?.timestamp_ns;
            if (new_ts !== undefined && (prev_ts === undefined || new_ts > prev_ts)) {
                entry.key = this.lastEntryKey;
                await this.lastLedgerEntry.put(entry, this.lastEntryKey);
            }
        }
        return entry.key;
    }

    // Method to add or update multiple entries at once
    async bulkAddOrUpdate(entries: LedgerEntry[]): Promise<void> {
        await this.ledgerEntries.bulkPut(entries);
        // Determine the entry with the highest timestamp from the new entries
        const maxEntry = entries.reduce((prev, curr) =>
            (!prev || (curr.parentBlockHash && curr.timestamp_ns && prev.timestamp_ns && curr.timestamp_ns > prev.timestamp_ns))
                ? curr
                : prev,
            null as LedgerEntry | null
        );

        console.debug('bulkAddOrUpdate setting max entry:', maxEntry);

        if (maxEntry) {
            const currentLast = await this.lastLedgerEntry.get(this.lastEntryKey);
            const new_ts = maxEntry?.timestamp_ns;
            const prev_ts = currentLast?.timestamp_ns;
            if (new_ts !== undefined && (prev_ts === undefined || new_ts > prev_ts)) {
                maxEntry.key = this.lastEntryKey;
                await this.lastLedgerEntry.put(maxEntry, this.lastEntryKey);
            }
        }
    }

    // Method to add or update a binary block
    async addOrUpdateBinaryBlock(blockOffset: number, data: Uint8Array): Promise<void> {
        await this.ledgerBinaryBlocks.put({
            blockOffset,
            data
        });
    }

    // Method to get a binary block by offset
    async getBinaryBlock(blockOffset: number): Promise<Uint8Array | undefined> {
        const block = await this.ledgerBinaryBlocks.get(blockOffset);
        return block?.data;
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
        await this.ledgerBinaryBlocks.clear();
    }

    // Method to get the ledger entry with the highest timestamp (quick access)
    async getLastEntry(): Promise<LedgerEntry | null> {
        return (await this.lastLedgerEntry.get(this.lastEntryKey)) || null;
    }

    // Method to get the highest block offset stored
    async getHighestBlockOffset(): Promise<number> {
        const lastEntry = await this.getLastEntry();
        if (lastEntry && lastEntry.blockOffset) {
            return lastEntry.blockOffset;
        }
        // If we don't have any entries yet, return 0 to start from the beginning
        return 0;
    }
}

// Create and export a singleton instance of the database
export const db = new LedgerDatabase();

/**
 * Fetches ledger blocks from the ledger canister and stores them in IndexedDB
 * - Fetches all blocks since the last stored block
 * - Processes the binary data through Rust WASM to split it into individual blocks
 * - Stores both the binary blocks and their JSON representation in IndexedDB
 *
 * @returns {Promise<number>} The number of new blocks processed
 */
export async function ledger_data_fetch(): Promise<number> {
    try {
        // Get the highest block offset we've stored so far
        const lastBlockOffset = await db.getHighestBlockOffset();
        console.log(`Last stored block offset: ${lastBlockOffset}`);

        // Create a cursor to fetch data from the ledger canister starting from where we left off
        const cursorString = `position=${lastBlockOffset}`;

        // Fetch data from the ledger canister
        console.log(`Fetching ledger data from cursor: ${cursorString}`);
        const result = await fetchDataWithCache(cursorString, null, false); // Don't bypass cache

        if (!result || !result.Ok || !Array.isArray(result.Ok) || result.Ok.length < 2) {
            console.warn("Invalid or empty response from ledger canister", result);
            return 0;
        }

        // Extract the cursor and data from the response
        const [remotePositionStr, binaryData] = result.Ok;

        if (!remotePositionStr || !binaryData || !(binaryData instanceof Uint8Array)) {
            console.warn("Invalid data format from canister", { remotePositionStr, binaryData });
            return 0;
        }

        // Parse the remote position
        const remotePosition = parseInt(remotePositionStr.split('=')[1], 10);

        if (isNaN(remotePosition) || remotePosition <= lastBlockOffset) {
            console.log("No new blocks available");
            return 0;
        }

        console.log(`Fetched ${binaryData.length} bytes of data, remote position: ${remotePosition}`);

        // Store the raw binary data at the appropriate offset
        await db.addOrUpdateBinaryBlock(lastBlockOffset, binaryData);

        // Parse and process each block using Rust WASM
        const newBlocks: LedgerEntry[] = [];
        let currentOffset = lastBlockOffset;

        // Loop until we've processed all the data or reached the remote position
        while (currentOffset < remotePosition) {
            try {
                // Use the exported function from client.js to parse the block at this offset into JSON
                const blockJsonStr = await ledger_get_block_as_json(BigInt(currentOffset));

                // Parse the JSON string into an object
                const blockData = JSON.parse(blockJsonStr);

                if (!blockData || !blockData.block_header) {
                    console.warn(`Invalid block data at offset ${currentOffset}`, blockData);
                    break;
                }

                // Process each entry in the block
                for (const entry of blockData.block) {
                    // Create a ledger entry for each item
                    const ledgerEntry: LedgerEntry = {
                        label: entry.label || "unknown",
                        key: entry.key || `block_${currentOffset}_${entry.index || 0}`,
                        value: entry.value,
                        description: entry.description || "",
                        timestamp_ns: blockData.block_header.timestamp_ns,
                        blockVersion: blockData.block_header.block_version,
                        blockSize: blockData.block.length,
                        parentBlockHash: blockData.block_header.parent_block_hash,
                        blockOffset: currentOffset,
                    };

                    newBlocks.push(ledgerEntry);
                }

                // Move to the next block
                // The next block starts after this block's data plus any alignment padding
                const nextOffset = blockData.block_header.offset + blockData.block_header.jump_bytes_next;

                if (nextOffset <= currentOffset) {
                    console.warn(`Invalid next offset ${nextOffset} <= current ${currentOffset}`);
                    break;
                }

                currentOffset = nextOffset;
            } catch (error) {
                console.error(`Error processing block at offset ${currentOffset}:`, error);
                break;
            }
        }

        // Store all the processed entries in IndexedDB
        if (newBlocks.length > 0) {
            console.log(`Storing ${newBlocks.length} new ledger entries`);
            await db.bulkAddOrUpdate(newBlocks);
        }

        return newBlocks.length;
    } catch (error) {
        console.error("Error in ledger_data_fetch:", error);
        throw error;
    }
}
