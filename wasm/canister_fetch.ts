import { HttpAgent, Actor } from '@dfinity/agent';
import { idlFactory } from './canister_idl.js';

import { db, LedgerEntry } from './db';
import { ledger_get_blocks_as_json } from './dc-client.js';

/**
 * Fetches ledger blocks from the ledger canister and stores them in IndexedDB
 * - Fetches all blocks since the last stored block
 * - Processes the binary data through Rust WASM to split it into individual blocks
 * - Stores the JSON representation in IndexedDB
 *
 * @returns {Promise<number>} The number of new blocks processed
 */
export async function ledger_data_fetch(): Promise<number> {
    try {
        // Step 1: Get the highest block offset we've stored locally
        const lastEntry = await db.getLastEntry();
        console.debug(`Fetching ledger data starting from last entry: ${lastEntry}`);

        // Create a cursor to fetch data from the ledger canister starting from where we left off
        let cursorString;
        let bytesBefore;
        if (lastEntry === null) {
            cursorString = `position=0`;
            bytesBefore = null;
        } else {
            cursorString = `position=${lastEntry.ledgerEntry.blockOffset}`;
            bytesBefore = lastEntry.bytesBefore;
        }

        // Step 2: Fetch the binary data from the ledger canister
        console.log(`Sending request to ledger canister with cursor: ${cursorString} and bytesBefore: ${bytesBefore}`);
        const result = await canisterQueryLedgerData(cursorString, bytesBefore);

        // Validate the response
        if (!result || !result.Ok || !Array.isArray(result.Ok) || result.Ok.length < 2) {
            console.warn("Invalid or empty response from ledger canister", result);
            return 0;
        }

        // Extract the cursor and binary data from the response
        const [remotePositionStr, binaryData] = result.Ok;

        if (!remotePositionStr || !binaryData || !(binaryData instanceof Uint8Array)) {
            console.warn("Invalid data format from canister", { remotePositionStr, binaryData });
            return 0;
        }
        console.debug(`Received remote position from ledger canister: ${remotePositionStr}`);
        console.debug(`Received binary data from ledger canister: ${binaryData.length} bytes`);

        // Parse the remote position
        const remotePosition = parseInt(remotePositionStr.split('=')[1], 10);
        const lastBlockOffset = lastEntry?.ledgerEntry.blockOffset || 0;

        if (isNaN(remotePosition) || remotePosition <= lastBlockOffset) {
            console.log("No new blocks available");
            return 0;
        }

        // Step 4: Process the binary data using the simplified WASM API
        console.log("Processing binary data into blocks using WASM...");
        const newBlocks: LedgerEntry[] = [];

        try {
            // Use the new WASM function to process all blocks at once
            const blockJsonStr = await ledger_get_blocks_as_json(binaryData, remotePosition);

            // Parse the JSON string into a JavaScript object - now an array of blocks
            const blocksData = JSON.parse(blockJsonStr);

            if (!Array.isArray(blocksData)) {
                console.warn(`Invalid blocks data, expected array but got:`, typeof blocksData);
                return 0;
            }

            console.log(`Successfully parsed ${blocksData.length} blocks from binary data`);

            // Process each block
            for (const blockData of blocksData) {
                if (!blockData || !blockData.block_header) {
                    console.warn(`Invalid block data:`, blockData);
                    continue;
                }

                const blockOffset = blockData.block_header.offset;

                // Process each entry in the block
                for (const entry of blockData.block) {
                    // Create a ledger entry for each item with proper metadata
                    const ledgerEntry: LedgerEntry = {
                        label: entry.label || "unknown",
                        key: entry.key || `block_${blockOffset}_${entry.index || 0}`,
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
        } catch (error) {
            console.error(`Error processing blocks:`, error);
        }

        // Step 5: Store the processed JSON entries in IndexedDB for frontend use
        if (newBlocks.length > 0) {
            console.log(`Storing ${newBlocks.length} new ledger entries in JSON format`);
            await db.bulkAddOrUpdate(newBlocks);
        }

        return newBlocks.length;
    } catch (error) {
        console.error("Error in ledger_data_fetch:", error);
        throw error;
    }
}


export async function canisterQueryLedgerData(cursor, bytesBefore) {
    // Add logging to help track cursor format issues
    try {
        // Sanitize cursor value to catch potential issues early
        if (typeof cursor !== 'string') {
            console.warn(`[canisterQueryLedgerData] Cursor is not a string, converting: ${cursor}`);
            cursor = String(cursor);
        }

        // Validate cursor format
        if (cursor.includes('undefined') || cursor.includes('null')) {
            console.warn(`[canisterQueryLedgerData] Suspicious cursor format: ${cursor}`);
        }
    } catch (e) {
        console.warn(`[canisterQueryLedgerData] Cursor format error: ${e.message}`);
    }

    console.log('[canisterQueryLedgerData] Fetching fresh data from canister');
    try {
        // Check binary data format
        if (bytesBefore) {
            console.log(
                `[canisterQueryLedgerData] bytesBefore type: ${typeof bytesBefore}, length: ${bytesBefore.length || 'unknown'}`
            );
        }

        // Wrap canister query with extra error handling
        let result;
        try {
            result = await queryCanister('data_fetch', [[cursor], [bytesBefore]], {});

            // Validate the result structure before processing
            if (result && result.Ok && Array.isArray(result.Ok)) {
                const binaryData = result.Ok[1];
                if (binaryData) {
                    console.log(
                        `[canisterQueryLedgerData] Binary data type: ${binaryData.constructor ? binaryData.constructor.name : 'unknown'}, length: ${binaryData.length || binaryData.byteLength || 'unknown'}`
                    );
                }
            }
        } catch (queryError) {
            console.error(`[Query] Error in data_fetch query: ${queryError.message}`, queryError);
            if (queryError.message && queryError.message.includes('TextDecoder')) {
                console.error('[canisterQueryLedgerData] TextDecoder error detected - possibly malformed binary data');
            }
            throw queryError;
        }

        console.log(
            `[canisterQueryLedgerData] Successfully fetched fresh data for cursor: ${cursor}`
        );
        return result;
    } catch (error) {
        console.error('Error in canisterQueryLedgerData:', error);
        throw error;
    }
}


export async function queryCanister(methodName, args, options = {}) {
    try {
        // Input validation
        if (!methodName || typeof methodName !== 'string') {
            throw new Error(`Invalid method name: ${methodName}`);
        }

        if (!Array.isArray(args)) {
            console.warn(`Args is not an array, converting: ${args}`);
            args = [args]; // Convert to array to avoid errors
        }

        // Get agent with better error handling
        let currentAgent;
        try {
            currentAgent = getAgent(null);
        } catch (agentError) {
            console.error('Failed to create agent:', agentError);
            throw new Error(`Agent creation failed: ${agentError.message}`);
        }

        const canisterId = options['canisterId'] || defaultConfig.canisterId;

        // Create actor with better error handling
        let actor;
        try {
            actor = Actor.createActor(idlFactory, {
                agent: currentAgent,
                canisterId,
            });
        } catch (actorError) {
            console.error('Failed to create actor:', actorError);
            throw new Error(`Actor creation failed: ${actorError.message}`);
        }

        if (typeof actor[methodName] !== 'function') {
            throw new Error(`Method ${methodName} not found on the canister interface.`);
        }

        // Call method with better error handling
        try {
            const result = await actor[methodName](...args);

            // Log success and return information about the result structure
            if (methodName === 'data_fetch' && result && result.Ok) {
                console.debug(
                    `[Cache] data_fetch result structure: ${JSON.stringify({
                        hasOk: !!result.Ok,
                        isArray: Array.isArray(result.Ok),
                        length: Array.isArray(result.Ok) ? result.Ok.length : 'not array',
                        firstElementType:
                            Array.isArray(result.Ok) && result.Ok[0] ? typeof result.Ok[0] : 'n/a',
                        secondElementExists: Array.isArray(result.Ok) && result.Ok.length > 1,
                    })}`
                );
            }

            return result;
        } catch (callError) {
            console.error(`Error calling method ${methodName}:`, callError);

            // Provide more diagnostics for TextDecoder errors
            if (callError.message && callError.message.includes('TextDecoder')) {
                console.error(
                    '[CRITICAL] TextDecoder.decode failed - this is likely due to malformed binary data',
                    {
                        errorName: callError.name,
                        errorStack: callError.stack,
                    }
                );
            }

            throw new Error(`Canister method call failed: ${callError.message}`);
        }
    } catch (error) {
        console.error('Error in queryCanister:', error);
        throw error;
    }
}

const defaultConfig = {
    networkUrl: 'https://icp-api.io',
    canisterId: 'ggi4a-wyaaa-aaaai-actqq-cai',
};

// Create agent lazily to avoid initialization issues
let agent = null;

function getAgent(identity) {
    if (!agent) {
        try {
            if (identity) {
                agent = HttpAgent.createSync({
                    host: defaultConfig.networkUrl,
                    shouldFetchRootKey: true,
                    identity,
                });
                console.log('Agent created with identity:', identity);
            } else {
                agent = HttpAgent.createSync({
                    host: defaultConfig.networkUrl,
                    shouldFetchRootKey: true,
                });
                console.log('Agent created without identity');
            }
        } catch (error) {
            console.error(`Failed to initialize ${identity || 'anonymous'} HttpAgent`);
            throw error;
        }
    }
    return agent;
}
