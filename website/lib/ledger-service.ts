import { DecentCloudClient, createClient } from '@decent-stuff/dc-client';
import { db, LedgerEntry } from './db';

// Define types for block data structures
interface BlockEntry {
    key: string;
    value: unknown;
    label?: string;
    description?: string;
    [key: string]: unknown; // Allow for additional properties
}

interface BlockHeader {
    block_version: number;
    jump_bytes_next: number;
    parent_block_hash: string;
    offset: number;
    timestamp_ns: number;
    [key: string]: unknown;
}

interface BlockResult {
    block?: BlockEntry[];
    block_header?: BlockHeader;
    Ok?: {
        block?: BlockEntry[];
        block_header?: BlockHeader;
    };
}

class LedgerService {
    private client: DecentCloudClient | null = null;
    private isInitialized = false;
    private pollingInterval: NodeJS.Timeout | null = null;
    private pollingFrequency = 10000; // 10 seconds by default

    // Initialize the ledger client
    async initialize(): Promise<boolean> {
        if (this.isInitialized) return true;

        try {
            // Create a new client instance using the createClient function
            this.client = createClient();
            await this.client.initialize();
            this.isInitialized = true;
            return true;
        } catch (error) {
            console.error('Failed to initialize Decent Cloud ledger client:', error);
            return false;
        }
    }

    // Start polling for ledger updates
    async startPolling(frequency?: number): Promise<void> {
        if (frequency) {
            this.pollingFrequency = frequency;
        }

        // Clear any existing polling interval
        if (this.pollingInterval) {
            clearInterval(this.pollingInterval);
        }

        // Fetch immediately on start and await completion
        try {
            await this.fetchAndStoreLatestEntries();
        } catch (error) {
            console.error('Initial fetch failed:', error);
            // Continue with polling even if initial fetch fails
        }

        // Set up new polling interval
        this.pollingInterval = setInterval(async () => {
            try {
                await this.fetchAndStoreLatestEntries();
            } catch (error) {
                console.error('Polling fetch failed:', error);
                // Continue polling even if a fetch fails
            }
        }, this.pollingFrequency);
    }

    // Check if polling is currently active
    isPollingActive(): boolean {
        return this.pollingInterval !== null;
    }

    // Check if the service is initialized
    getInitializationStatus(): boolean {
        return this.isInitialized;
    }

    // Stop polling for ledger updates
    stopPolling(): void {
        if (this.pollingInterval) {
            clearInterval(this.pollingInterval);
            this.pollingInterval = null;
        }
    }

    // Fetch the latest ledger entries and store them in the database
    async fetchAndStoreLatestEntries(): Promise<LedgerEntry[]> {
        if (!this.isInitialized || !this.client) {
            const initialized = await this.initialize();
            if (!initialized) {
                throw new Error('Ledger client is not initialized');
            }
        }

        try {
            // Start from block 0 and fetch as many blocks as possible
            const ledgerEntries: LedgerEntry[] = [];
            let blockOffset = BigInt(0);
            let hasMoreBlocks = true;

            // Fetch blocks until we encounter an error or reach a limit
            const MAX_BLOCKS = 10000; // Limit to prevent infinite loops
            let blockCount = 0;

            while (hasMoreBlocks && blockCount < MAX_BLOCKS) {
                try {
                    // Get block + block header as JSON string
                    const blockResult = this.client!.ledger.getBlockAsJson(blockOffset);

                    // Parse the result
                    let parsedResult: BlockResult;
                    if (typeof blockResult === 'string') {
                        parsedResult = JSON.parse(blockResult) as BlockResult;
                    } else {
                        parsedResult = blockResult as BlockResult;
                    }
                    // Extract the block contents
                    let block: BlockEntry[] | undefined;
                    let blockHeader: BlockHeader | undefined;
                    if (parsedResult.Ok) {
                        parsedResult = parsedResult.Ok
                    }
                    if (parsedResult.block) {
                        block = parsedResult.block;
                    }
                    if (parsedResult.block_header) {
                        blockHeader = parsedResult.block_header;
                    }

                    if (block && blockHeader) {
                        for (const blockEntry of block) {
                            const entry: LedgerEntry = {
                                label: blockEntry.label || '', // Provide default empty string for label
                                key: blockEntry.key,
                                value: blockEntry.value,
                                description: blockEntry.description || '', // Provide default empty string for description
                                blockVersion: blockHeader.block_version,
                                blockSize: blockHeader.jump_bytes_next,
                                parentBlockHash: blockHeader.parent_block_hash,
                                blockOffset: blockHeader.offset,
                                timestamp_ns: blockHeader.timestamp_ns / 1000000
                            };
                            ledgerEntries.push(entry);
                        }
                        blockOffset = BigInt(blockHeader.offset) + BigInt(blockHeader.jump_bytes_next);
                        if (blockHeader.jump_bytes_next === 0) { // Fixed property name from blockSize to jump_bytes_next
                            hasMoreBlocks = false;
                        }
                    } else {
                        console.log(`No block found at offset ${blockOffset}`);
                        hasMoreBlocks = false;
                    }
                    blockCount++;
                } catch (err) {
                    console.log(`Reached end of blocks or error at offset ${blockOffset}:`, err);
                    hasMoreBlocks = false;
                }
            }

            if (ledgerEntries.length > 0) {
                // Store the entries in the database
                await db.bulkAddOrUpdate(ledgerEntries);
            }

            return ledgerEntries;
        } catch (error) {
            console.error('Failed to fetch ledger entries:', error);
            throw error;
        }
    }

    // Get all entries from the local database
    async getAllEntries(): Promise<LedgerEntry[]> {
        return await db.getAllEntries();
    }

    // Get the parent block hash for the last entry in the ledger
    async getLastEntryParentBlockHash(): Promise<string | null> {
        try {
            // Try to fetch new entries
            await this.fetchAndStoreLatestEntries();

            // Then get the last entry
            const entry = await db.getLastEntry();
            console.info(`Last entry: ${JSON.stringify(entry)}`);

            if (entry?.parentBlockHash) {
                const decodedHash = Buffer.from(entry.parentBlockHash, 'base64').toString('hex');
                return decodedHash;
            }
            return null;
        } catch (error) {
            console.error("Error getting parent block hash:", error);
            return null;
        }
    }

    async getNextBlockOffset(): Promise<bigint | null> {
        try {
            const entry = await db.getLastEntry();
            if (entry) {
                return BigInt(entry.blockOffset) + BigInt(entry.blockSize);
            }
            return null;
        } catch (error) {
            console.error("Error getting next block offset:", error);
            return null;
        }
    }

    // Get a specific entry by key from the local database
    async getEntry(key: string): Promise<LedgerEntry | undefined> {
        return await db.getEntry(key);
    }

    // Clear all entries from the local database
    async clearAllEntries(): Promise<void> {
        await db.clearAllEntries();
    }

    // Disconnect the client
    disconnect(): void {
        this.stopPolling();
        // Just set the client to null since there's no explicit disconnect method
        this.client = null;
        this.isInitialized = false;
    }
}

// Create and export a singleton instance of the service
export const ledgerService = new LedgerService();
