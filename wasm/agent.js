import { HttpAgent, Actor } from '@dfinity/agent';
import { idlFactory } from './canister_idl.js';
/**
 * Default configuration for the agent
 */
const defaultConfig = {
    networkUrl: 'https://icp-api.io',
    canisterId: 'ggi4a-wyaaa-aaaai-actqq-cai',
};
// Singleton agent instance
let agent = null;
/**
 * Configure the agent with custom settings
 * @param config Configuration options
 */
export function configure(config) {
    Object.assign(defaultConfig, config);
    agent = null; // Reset the agent to force recreation with new config
}
/**
 * Get or create an agent instance
 * @param identity Optional identity to use for the agent
 * @returns An HttpAgent instance
 */
export function getAgent(identity) {
    if (!agent) {
        try {
            if (identity) {
                agent = HttpAgent.createSync({
                    host: defaultConfig.networkUrl,
                    shouldFetchRootKey: true,
                    identity,
                });
                console.log('Agent created with identity:', identity);
            }
            else {
                agent = HttpAgent.createSync({
                    host: defaultConfig.networkUrl,
                    shouldFetchRootKey: true,
                });
                console.log('Agent created without identity');
            }
        }
        catch (error) {
            console.error(`Failed to initialize ${identity ? 'authenticated' : 'anonymous'} HttpAgent`);
            throw error;
        }
    }
    return agent;
}
/**
 * Query a canister method
 * @param methodName The name of the method to call
 * @param args The arguments to pass to the method
 * @param options Additional options
 * @returns The result of the query
 */
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
        }
        catch (agentError) {
            console.error('Failed to create agent:', agentError);
            throw new Error(`Agent creation failed: ${agentError.message}`);
        }
        const canisterId = options.canisterId || defaultConfig.canisterId;
        // Create actor with better error handling
        let actor;
        try {
            actor = Actor.createActor(idlFactory, {
                agent: currentAgent,
                canisterId,
            });
        }
        catch (actorError) {
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
                console.debug(`[Cache] data_fetch result structure: ${JSON.stringify({
                    hasOk: !!result.Ok,
                    isArray: Array.isArray(result.Ok),
                    length: Array.isArray(result.Ok) ? result.Ok.length : 'not array',
                    firstElementType: Array.isArray(result.Ok) && result.Ok[0] ? typeof result.Ok[0] : 'n/a',
                    secondElementExists: Array.isArray(result.Ok) && result.Ok.length > 1,
                })}`);
            }
            return result;
        }
        catch (callError) {
            console.error(`Error calling method ${methodName}:`, callError);
            // Provide more diagnostics for TextDecoder errors
            const errorMessage = callError.message;
            if (errorMessage && errorMessage.includes('TextDecoder')) {
                console.error('[CRITICAL] TextDecoder.decode failed - this is likely due to malformed binary data', {
                    errorName: callError.name,
                    errorStack: callError.stack,
                });
            }
            throw new Error(`Canister method call failed: ${errorMessage}`);
        }
    }
    catch (error) {
        console.error('Error in queryCanister:', error);
        throw error;
    }
}
/**
 * Update a canister method (authenticated call)
 * @param methodName The name of the method to call
 * @param args The arguments to pass to the method
 * @param identity The identity to use for the call
 * @param options Additional options
 * @returns The result of the update
 */
export async function updateCanister(methodName, args, identity, options = {}) {
    try {
        const currentAgent = getAgent(identity);
        const canisterId = options.canisterId || defaultConfig.canisterId;
        const actor = Actor.createActor(idlFactory, {
            agent: currentAgent,
            canisterId,
        });
        if (typeof actor[methodName] !== 'function') {
            throw new Error(`Method "${methodName}" not found on the canister interface.`);
        }
        return await actor[methodName](...args);
    }
    catch (error) {
        console.error('Error in updateCanister:', error);
        throw error;
    }
}
/**
 * Query the ledger canister for new blocks
 * @param cursor The cursor position to start fetching from
 * @param bytesBefore Optional bytes before the cursor
 * @returns The result of the query
 */
export async function canisterQueryLedgerData(cursor, bytesBefore) {
    // Add logging to help track cursor format issues
    try {
        // Sanitize cursor value to catch potential issues early
        if (typeof cursor !== 'string') {
            console.warn(`[Fetch] Cursor is not a string, converting: ${cursor}`);
            cursor = String(cursor);
        }
        // Validate cursor format
        if (cursor.includes('undefined') || cursor.includes('null')) {
            console.warn(`[Fetch] Suspicious cursor format: ${cursor}`);
        }
    }
    catch (e) {
        console.warn(`[Fetch] Cursor format error: ${e.message}`);
    }
    console.log('[Fetch] Fetching data from canister');
    try {
        // Check binary data format
        if (bytesBefore) {
            console.log(`[Fetch] bytesBefore type: ${typeof bytesBefore}, length: ${bytesBefore.length || 'unknown'}`);
        }
        // Wrap canister query with extra error handling
        let result;
        try {
            result = await queryCanister('data_fetch', [[cursor], bytesBefore || []], {});
            // Validate the result structure before processing
            if (result && result.Ok && Array.isArray(result.Ok)) {
                const binaryData = result.Ok[1];
                if (binaryData) {
                    console.log(`[Fetch] Binary data type: ${binaryData.constructor ? binaryData.constructor.name : 'unknown'}, length: ${binaryData.length || binaryData.byteLength || 'unknown'}`);
                }
            }
        }
        catch (queryError) {
            console.error(`[Fetch] Error in data_fetch query: ${queryError.message}`, queryError);
            const errorMessage = queryError.message;
            if (errorMessage && errorMessage.includes('TextDecoder')) {
                console.error('[Fetch] TextDecoder error detected - possibly malformed binary data');
            }
            throw queryError;
        }
        console.log(`[Fetch] Successfully fetched fresh data for cursor: ${cursor}`);
        return result;
    }
    catch (error) {
        console.error('Error in Fetch:', error);
        throw error;
    }
}
