import { db, LedgerEntry, LastLedgerEntry } from './db';
import { canisterQueryLedgerData } from './agent';
import { parseLedgerBlocks } from './dc-client.js';

class Ledger {
    /**
     * Initialize the ledger interface.
     * This includes initializing the WASM module, the database (if necessary),
     * and fetching new ledger entries from the remote (canister) ledger.
     */
    async init(): Promise<void> {
        console.log("Initializing Ledger interface...");
        // If your db module requires explicit initialization, call it here:
        // await db.initialize();

        // Initialize the WASM module and fetch new ledger blocks.
        await this.fetchLedgerBlocks();
        console.log("Ledger interface initialization complete.");
    }

    /**
     * Fetch new ledger blocks from the remote ledger canister, process them with the WASM module,
     * and store the resulting ledger entries in the local database.
     *
     * @returns {Promise<number>} The number of new ledger entries processed.
     */
    async fetchLedgerBlocks(): Promise<number> {
        try {
            // Step 1: Get the highest block offset stored locally.
            const lastEntry = await db.getLastEntry();
            console.debug("Fetching ledger data starting from last entry:", lastEntry);

            // Step 2: Create a cursor for fetching data.
            let cursorString: string;
            let bytesBefore: Uint8Array | undefined;
            if (lastEntry === null) {
                cursorString = "position=0";
                bytesBefore = undefined;
            } else {
                cursorString = `position=${lastEntry.ledgerEntry.blockOffset}`;
                bytesBefore = lastEntry.bytesBefore;
            }

            console.log(
                `Requesting ledger canister with cursor: ${cursorString} and bytesBefore: ${bytesBefore ? "present" : "not present"}`
            );
            const result = await canisterQueryLedgerData(cursorString, bytesBefore);

            // Validate the response.
            if (!result || !result.Ok || !Array.isArray(result.Ok) || result.Ok.length < 2) {
                console.warn("Invalid or empty response from ledger canister", result);
                return 0;
            }

            const [remotePositionStr, binaryData] = result.Ok;
            if (!remotePositionStr || !binaryData || !(binaryData instanceof Uint8Array)) {
                console.warn("Invalid data format from canister", { remotePositionStr, binaryData });
                return 0;
            }

            console.debug("Received remote position:", remotePositionStr);
            console.debug("Received binary data:", binaryData.length, "bytes");

            // Parse the remote position and compare with the last stored block.
            const remotePosition = parseInt(remotePositionStr.split("=")[1], 10);
            const lastBlockOffset = lastEntry?.ledgerEntry.blockOffset || 0;
            if (isNaN(remotePosition) || remotePosition <= lastBlockOffset) {
                console.log("No new blocks available");
                return 0;
            }

            // Step 3: Process the binary data using the WASM function.
            console.log("Processing binary data into ledger blocks using WASM...");
            const newBlocks: LedgerEntry[] = [];

            try {
                const blocksData = await parseLedgerBlocks(binaryData, BigInt(lastBlockOffset));
                if (!Array.isArray(blocksData)) {
                    console.warn("Invalid blocks data; expected an array but got:", typeof blocksData);
                    return 0;
                }
                console.log(`Parsed ${blocksData.length} blocks from binary data`);

                // Process each block and its entries.
                for (const blockData of blocksData) {
                    if (!blockData || !blockData.block_header) {
                        console.warn("Invalid block data:", blockData);
                        continue;
                    }
                    const blockOffset = blockData.block_header.offset;
                    for (const entry of blockData.block) {
                        const ledgerEntry: LedgerEntry = {
                            label: entry.label || "unknown",
                            key: (entry.key as string) || `block_${blockOffset}_${entry.index || 0}`,
                            value: entry.value,
                            description: entry.description || "",
                            timestamp_ns: blockData.block_header.timestamp_ns,
                            blockVersion: blockData.block_header.block_version,
                            blockSize: blockData.block.length,
                            parentBlockHash: blockData.block_header.parent_block_hash,
                            blockOffset: blockOffset,
                        };
                        newBlocks.push(ledgerEntry);
                    }
                }

                // Store the last few bytes of the binary data for the next fetch.
                if (binaryData.length > 0) {
                    const bytesBeforeLength = Math.min(32, binaryData.length);
                    const newBytesBefore = binaryData.slice(binaryData.length - bytesBeforeLength);
                    await db.updateLastEntryBytes(newBytesBefore);
                }
            } catch (error) {
                console.error("Error processing blocks with WASM:", error);
                return 0;
            }

            // Step 4: Store the new ledger entries in the local database.
            if (newBlocks.length > 0) {
                console.log(`Storing ${newBlocks.length} new ledger entries in IndexedDB`);
                await db.bulkAddOrUpdate(newBlocks);
            }

            return newBlocks.length;
        } catch (error) {
            console.error("Error in fetchLedgerBlocks:", error);
            throw error;
        }
    }

    /**
     * Retrieve all ledger entries stored in the local database.
     *
     * @returns {Promise<LedgerEntry[]>} An array of all ledger entries.
     */
    async getAllEntries(): Promise<LedgerEntry[]> {
        try {
            return await db.getAllEntries();
        } catch (error) {
            console.error("Error retrieving all ledger entries:", error);
            throw error;
        }
    }

    /**
     * Retrieve the last fetched ledger block entry.
     *
     * @returns {Promise<LedgerEntry | null>} The last ledger entry or null if none exists.
     */
    async getLastFetchedBlock(): Promise<LastLedgerEntry | null> {
        try {
            return await db.getLastEntry();
        } catch (error) {
            console.error("Error retrieving the last fetched ledger entry:", error);
            throw error;
        }
    }

    /**
     * Clear the ledger storage.
     */
    async clearStorage(): Promise<void> {
        try {
            await db.clearAllEntries();
        } catch (error) {
            console.error("Error clearing the ledger storage:", error);
            throw error;
        }
    }
}

// Export a singleton instance of the Ledger.
export const ledger = new Ledger();
