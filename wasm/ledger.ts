import { db, LedgerBlock, LedgerEntry } from './db';
import { canisterQueryLedgerData } from './agent';
import { parseLedgerBlocks } from './dc-client.js';
import { providerLabelVariants } from './labels';

// Types for ledger transaction entries
interface Account {
    owner: string;
    subaccount: string | null;
}

interface TransactionV1 {
    from: Account;
    to: Account;
    created_at_time: number;
    memo: string;
    amount: number;
    balance_from_after: number;
    balance_to_after: number;
}

interface LedgerTransactionEntry {
    V1: TransactionV1;
}

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
        const MAX_FETCH_ITERATIONS = 50;

        return this.withErrorHandling(
            "fetching ledger blocks",
            async () => {
                if (!db.isAvailable()) {
                    const s = 'IndexedDB API unavailable in this runtime; skipping ledger fetch.';
                    console.warn(s);
                    db.setError(s);
                    return s;
                }

                // Step 1: Determine where to resume fetching from.
                const lastBlock = await db.getLastBlock();
                let cursorPosition = lastBlock?.fetchOffset ?? 0;
                let bytesBefore: [Uint8Array] | undefined = lastBlock
                    ? [base64ToUint8Array(lastBlock.fetchCompareBytes)]
                    : undefined;

                let totalBlocks = 0;
                let totalEntries = 0;
                let iteration = 0;

                while (true) {
                    iteration += 1;
                    const cursorString = `position=${cursorPosition}`;

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

                    // Parse metadata from the response string.
                    // Example: "position=8388608&response_bytes=143991&direction=forward&more=false"
                    const remotePositionMatch = remotePositionStr.match(/position=(\d+)/);
                    const remotePosition = remotePositionMatch ? parseInt(remotePositionMatch[1], 10) : NaN;
                    const moreMatch = remotePositionStr.match(/more=(true|false)/i);
                    const hasMore = moreMatch ? moreMatch[1].toLowerCase() === 'true' : false;

                    console.debug(`Received remote position: ${remotePosition} from str ${remotePositionStr}`);
                    console.debug("Received binary data:", binaryData.length, "bytes");

                    if (binaryData.length === 0) {
                        if (totalBlocks === 0) {
                            const s = `Fetch successful, no new ledger data found.`;
                            console.info(s);
                            return s;
                        }
                        console.info('No additional binary data returned; stopping incremental fetch loop.');
                        break;
                    }

                    // Step 3: Process the binary data using the WASM function.
                    console.log("Processing binary data into ledger blocks using WASM...");
                    const newBlocks: LedgerBlock[] = [];
                    const newEntries: LedgerEntry[] = [];

                    try {
                        const blocksData = await parseLedgerBlocks(
                            binaryData,
                            BigInt(Number.isNaN(remotePosition) ? cursorPosition : remotePosition)
                        );
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

                    if (newBlocks.length === 0) {
                        console.warn(
                            'No ledger blocks were parsed from the response despite receiving binary data; stopping fetch loop to avoid infinite retry.'
                        );
                        break;
                    }

                    console.log(
                        `Storing ${newBlocks.length} new blocks and ${newEntries.length} new ledger entries in IndexedDB`
                    );
                    await db.bulkAddOrUpdate(newBlocks, newEntries);

                    totalBlocks += newBlocks.length;
                    totalEntries += newEntries.length;

                    const lastFetchedBlock = newBlocks[newBlocks.length - 1];
                    cursorPosition = Number.isNaN(remotePosition)
                        ? lastFetchedBlock.fetchOffset
                        : remotePosition;
                    bytesBefore = [base64ToUint8Array(lastFetchedBlock.fetchCompareBytes)];

                    if (!hasMore) {
                        console.info('Ledger canister indicated no more data to fetch.');
                        break;
                    }

                    if (iteration >= MAX_FETCH_ITERATIONS) {
                        console.error(
                            `Reached maximum ledger fetch iterations (${MAX_FETCH_ITERATIONS}); Stopping to prevent infinite loop.`
                        );
                        break;
                    }
                }

                if (totalBlocks === 0) {
                    const s = `Fetch successful, no new ledger data found.`;
                    console.info(s);
                    return s;
                }

                return `Fetched ${totalBlocks} new blocks and ${totalEntries} new ledger entries.`;
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

    // Check if a particular provider principal is registered as a provider
    async isProviderRegistered(principal: string): Promise<boolean> {
        console.log("Checking if provider is registered:", principal);
        return this.withErrorHandling(
            "checking if provider is registered",
            async () => {
                const entries = await db.getEntriesByLabelAndKey(
                    providerLabelVariants.register,
                    principal
                );
                console.log(`Found ${entries.length} registration entries for principal ${principal}`);
                return entries.length > 0;
            },
            false
        );
    }

    /**
     * Get the balance of the specified account
     * @param owner The owner of the account
     * @param subaccount The subaccount of the account
     * @returns {Promise<number>} The balance of the specified account
     */
    async getAccountBalance(owner: string | null, subaccount: string | null): Promise<number> {
        return this.withErrorHandling(
            "retrieving balance",
            async () => {
                if (!owner) return 0;
                const entries: unknown[] = await db.getEntriesByLabel("DCTokenTransfer");
                // Iterate in reverse order to find the latest balance
                for (const rawEntry of entries.reverse()) {
                    const entry = rawEntry as LedgerTransactionEntry;
                    /* We are looking for balance_from_after or balance_to_after if the account is the "from" or "to"
                    {
                        "V1": {
                            "from": {
                                "owner": "yp4qz-xtz2x-66yql-yymle-guxye-matt3-os7u7-ctzvp-oig6d-feehe-6qe",
                                "subaccount": null
                            },
                            "to": {
                                "owner": "zjbo4-sknjf-hfisk-oi4",
                                "subaccount": null
                            },
                            "fee": 500000000,
                            "fees_accounts": [
                                {
                                    "owner": "zjbo4-sknjf-hfisk-oi4",
                                    "subaccount": null
                                }
                            ],
                            "created_at_time": 1742554245399975000,
                            "memo": "check-in-yp4qz-287-Website Valid",
                            "amount": 0,
                            "balance_from_after": 158055499999997,
                            "balance_to_after": 0
                        }
                    }
                    */
                    if (entry.V1 === undefined || entry.V1 === null) {
                        continue;
                    }
                    if (entry.V1.from.owner === owner && entry.V1.from.subaccount === subaccount) {
                        return entry.V1.balance_from_after;
                    } else if (entry.V1.to.owner === owner && entry.V1.to.subaccount === subaccount) {
                        return entry.V1.balance_to_after;
                    }
                }
                return 0;
            },
            0
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
