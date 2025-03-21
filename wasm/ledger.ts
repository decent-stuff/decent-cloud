import { db, LedgerBlock, LedgerEntry } from './db';
import { canisterQueryLedgerData } from './agent';
import { parseLedgerBlocks } from './dc-client.js';

class DecentCloudLedger {
    /**
     * Utility method to handle operations with consistent error handling
     * @param operationName Name of the operation for error messages
     * @param operation Function that performs the operation
     * @param defaultValue Optional default value to return in case of error
     * @returns Result of the operation or defaultValue in case of error
     */
    private async withErrorHandling<T>(
        operationName: string,
        operation: () => Promise<T>,
        defaultValue: T
    ): Promise<T>;
    private async withErrorHandling<T>(
        operationName: string,
        operation: () => Promise<T>
    ): Promise<T>;
    private async withErrorHandling<T>(
        operationName: string,
        operation: () => Promise<T>,
        defaultValue?: T
    ): Promise<T> {
        // Clear any previous database error
        db.setError(null);

        try {
            // Execute the operation
            return await operation();
        } catch (error) {
            console.error(`Error ${operationName}:`, error);

            // Set the database error
            if (error instanceof Error) {
                db.setError(`Error ${operationName}: ${error.message}`);
            } else {
                db.setError(`Error ${operationName}: ${String(error)}`);
            }

            // Return default value or rethrow based on whether defaultValue is provided
            if (arguments.length >= 3) {
                return defaultValue as T;
            }
            throw error;
        }
    }

    /**
     * Initialize the ledger interface and fetch new ledger
     * entries from the remote (canister) ledger.
     */
    async init(): Promise<void> {
        console.log("Initializing Ledger interface...");

        return this.withErrorHandling(
            "initializing ledger interface",
            async () => {
                // If your db module requires explicit initialization, call it here:
                // await db.initialize();

                await this.fetchLedgerBlocks();
                console.log("Ledger interface initialization complete.");
            }
        );
    }

    /**
     * Fetch new ledger blocks from the remote ledger canister, process them with the WASM module,
     * and store the resulting ledger entries in the local database.
     *
     * @returns {Promise<string>} Fetch result.
     */
    async fetchLedgerBlocks(): Promise<string> {
        return this.withErrorHandling(
            "fetching ledger blocks",
            async () => {
                // Step 1: Get the highest block offset stored locally.
                const lastBlock = await db.getLastBlock();

                // Step 2: Create a cursor for fetching data.
                let cursorString: string;
                let bytesBefore: [Uint8Array] | undefined;
                if (lastBlock === null) {
                    cursorString = "position=0";
                    bytesBefore = undefined;
                } else {
                    cursorString = `position=${lastBlock.fetchOffset}`;
                    bytesBefore = [base64ToUint8Array(lastBlock.fetchCompareBytes)];
                }

                console.log(
                    `Requesting ledger canister with cursor: ${cursorString} and bytesBefore: ${bytesBefore}`
                );
                const result = await canisterQueryLedgerData(cursorString, bytesBefore);

                // Validate the response.
                if (!result || !result.Ok || !Array.isArray(result.Ok) || result.Ok.length < 2) {
                    const s = `Invalid or empty response from ledger canister: ${result}`;
                    console.warn(s);
                    return s;
                }

                const [remotePositionStr, binaryData] = result.Ok;
                if (!remotePositionStr || !binaryData || !(binaryData instanceof Uint8Array)) {
                    const s = `Invalid data format from canister: ${remotePositionStr}, ${binaryData}`;
                    console.warn(s);
                    return s;
                }

                // Parse the remote position and compare with the last stored block.
                // Example of remotePositionStr:
                // "position=8388608&response_bytes=143991&direction=forward&more=false"
                const remotePositionMatch = remotePositionStr.match(/position=(\d+)/);
                const remotePosition = remotePositionMatch ? parseInt(remotePositionMatch[1], 10) : NaN;

                console.debug(`Received remote position: ${remotePosition} from str ${remotePositionStr}`);
                console.debug("Received binary data:", binaryData.length, "bytes");

                if (binaryData.length === 0) {
                    const s = `Fetch successful, no new ledger data found.`;
                    console.info(s);
                    return s;
                }

                // Step 3: Process the binary data using the WASM function.
                console.log("Processing binary data into ledger blocks using WASM...");
                const newBlocks: LedgerBlock[] = [];
                const newEntries: LedgerEntry[] = [];

                try {
                    const blocksData = await parseLedgerBlocks(binaryData, BigInt(remotePosition));
                    if (!Array.isArray(blocksData)) {
                        const s = `Invalid data format from canister: ${blocksData}`;
                        console.warn(s);
                        return s;
                    }
                    console.log(`Parsed ${blocksData.length} blocks from binary data`);

                    // Process each block and its entries.
                    for (const blockData of blocksData) {
                        if (!blockData || !blockData.block_header) {
                            console.warn("Invalid block data:", blockData);
                            continue;
                        }

                        const blockHeader: LedgerBlock = {
                            blockVersion: blockData.block_header.block_version,
                            blockSize: blockData.block.length,
                            parentBlockHash: blockData.block_header.parent_block_hash,
                            blockHash: blockData.block_header.block_hash,
                            blockOffset: blockData.block_header.offset,
                            fetchCompareBytes: blockData.block_header.fetch_compare_bytes,
                            fetchOffset: blockData.block_header.fetch_offset,
                            timestampNs: blockData.block_header.timestamp_ns
                        };
                        newBlocks.push(blockHeader);

                        for (const entry of blockData.block) {
                            if (!entry.label || !entry.key) {
                                console.warn("Invalid entry data:", entry);
                                continue;
                            }
                            const ledgerEntry: LedgerEntry = {
                                blockOffset: blockData.block_header.offset,
                                label: entry.label,
                                key: entry.key,
                                value: entry.value,
                                description: entry.description
                            };
                            newEntries.push(ledgerEntry);
                        }
                    }
                } catch (error) {
                    const s = `Error processing blocks with WASM: ${error}`;
                    console.warn(s);
                    return s;
                }

                // Step 4: Store the new ledger entries in the local database.
                if (newBlocks.length > 0) {
                    console.log(`Storing ${newBlocks.length} new blocks and ${newEntries.length} new ledger entries in IndexedDB`);
                    await db.bulkAddOrUpdate(newBlocks, newEntries);
                }

                return `Fetched ${newBlocks.length} new blocks and ${newEntries.length} new ledger entries.`;
            },
            "Error fetching ledger blocks. See console for details."
        );
    }

    /**
     * Retrieve all ledger entries stored in the local database.
     *
     * @returns {Promise<LedgerEntry[]>} An array of all ledger entries.
     */
    async getAllEntries(): Promise<LedgerEntry[]> {
        return this.withErrorHandling(
            "retrieving all ledger entries",
            async () => await db.getAllEntries(),
            []
        );
    }

    // Get all blocks from the ledger
    async getAllBlocks(): Promise<LedgerBlock[]> {
        return this.withErrorHandling(
            "retrieving all ledger blocks",
            async () => await db.getAllBlocks(),
            []
        );
    }

    /**
     * Retrieve entries for a specific block.
     *
     * @param blockOffset The offset of the block to retrieve entries for.
     * @returns {Promise<LedgerEntry[]>} An array of ledger entries for the specified block.
     */
    async getBlockEntries(blockOffset: number): Promise<LedgerEntry[]> {
        return this.withErrorHandling(
            `retrieving ledger entries for block ${blockOffset}`,
            async () => await db.getBlockEntries(blockOffset),
            []
        );
    }

    /**
     * Retrieve the last fetched ledger block entry.
     *
     * @returns {Promise<LedgerEntry | null>} The last ledger entry or null if none exists.
     */
    async getLastFetchedBlock(): Promise<LedgerBlock | null> {
        return this.withErrorHandling(
            "retrieving the last fetched ledger block",
            async () => await db.getLastBlock(),
            null
        );
    }

    /**
     * Clear the ledger storage.
     */
    async clearStorage(): Promise<void> {
        return this.withErrorHandling(
            "clearing the ledger storage",
            async () => await db.clearAllEntries()
        );
    }
}

/**
 * Converts a base64 string to a Uint8Array
 * @param b64string The base64 string to convert
 * @returns The resulting Uint8Array
 */
export function base64ToUint8Array(b64string: string): Uint8Array {
    const binaryString = atob(b64string);
    const len = binaryString.length;
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) {
        bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes;
}

// Export a singleton instance of the Ledger.
export const decentCloudLedger = new DecentCloudLedger();
