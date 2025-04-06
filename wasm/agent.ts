import { HttpAgent, Actor, Identity } from '@dfinity/agent';
import { idlFactory } from './canister_idl.js';

/**
 * Default configuration for the agent
 */
const defaultConfig = {
    networkUrl: 'https://icp-api.io',
    canisterId: 'ggi4a-wyaaa-aaaai-actqq-cai',
};

// Singleton agent instance
let agent: HttpAgent | null = null;
let agentIdentity: Identity | null = null;

/**
 * Get or create an agent instance
 * @param identity Optional identity to use for the agent
 * @returns An HttpAgent instance
 */
export function getAgent(identity?: Identity | null): HttpAgent {
    // Create new agent if there isn't one or the identity differs
    if (!agent || agentIdentity !== identity) {
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
            // Store the identity used for this agent
            agentIdentity = identity || null;
        } catch (error) {
            console.error(`Failed to initialize ${identity ? 'authenticated' : 'anonymous'} HttpAgent`);
            throw error;
        }
    }
    return agent;
}

/**
 * Configure the agent with custom settings
 * @param config Configuration options
 */
export function configure(config: Partial<typeof defaultConfig>): void {
    Object.assign(defaultConfig, config);
    agent = null; // Reset the agent to force recreation with new config
}

/**
 * Type for canister query options
 */
interface CanisterCallOptions {
    canisterId?: string;
}

/**
 * Query a canister method
 * @param methodName The name of the method to call
 * @param args The arguments to pass to the method
 * @param options Additional options
 * @returns The result of the query
 */
export async function queryCanister(
    methodName: string,
    args: unknown[],
    options: CanisterCallOptions = {}
): Promise<unknown> {
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
            throw new Error(`Agent creation failed: ${(agentError as Error).message}`);
        }

        const canisterId = options.canisterId || defaultConfig.canisterId;

        // Create actor with better error handling
        let actor;
        try {
            actor = Actor.createActor(idlFactory, {
                agent: currentAgent,
                canisterId,
            });
        } catch (actorError) {
            console.error('Failed to create actor:', actorError);
            throw new Error(`Actor creation failed: ${(actorError as Error).message}`);
        }

        if (typeof actor[methodName] !== 'function') {
            throw new Error(`Method ${methodName} not found on the canister interface.`);
        }

        // Call method with better error handling
        try {
            return await actor[methodName](...args);
        } catch (callError) {
            console.error(`Error calling method ${methodName}:`, callError);

            // Provide more diagnostics for TextDecoder errors
            const errorMessage = (callError as Error).message;
            if (errorMessage && errorMessage.includes('TextDecoder')) {
                console.error(
                    '[CRITICAL] TextDecoder.decode failed - this is likely due to malformed binary data',
                    {
                        errorName: (callError as Error).name,
                        errorStack: (callError as Error).stack,
                    }
                );
            }

            throw new Error(`Canister method call failed: ${errorMessage}`);
        }
    } catch (error) {
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
export async function updateCanister(
    methodName: string,
    args: unknown[],
    identity: Identity,
    options: CanisterCallOptions = {}
): Promise<unknown> {
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
    } catch (error) {
        console.error('Error in updateCanister:', error);
        throw error;
    }
}

/**
 * Type for ledger data response
 */
interface LedgerDataResponse {
    Ok?: [string, Uint8Array];
    Err?: string;
}

/**
 * Query the ledger canister for new blocks
 * @param cursor The cursor position to start fetching from
 * @param bytesBefore Optional bytes before the cursor
 * @returns The result of the query
 */
export async function canisterQueryLedgerData(cursor: string, bytesBefore?: [Uint8Array]): Promise<LedgerDataResponse> {
    console.log('[Fetch] Fetching data from canister, with cursor:', cursor);
    try {
        // Wrap canister query with extra error handling
        let result: LedgerDataResponse;
        try {
            result = await queryCanister('data_fetch', [[cursor], bytesBefore || []], {}) as LedgerDataResponse;
        } catch (queryError) {
            console.error(`[Fetch] Error in data_fetch query: ${(queryError as Error).message}`, queryError);
            const errorMessage = (queryError as Error).message;
            if (errorMessage && errorMessage.includes('TextDecoder')) {
                console.error('[Fetch] TextDecoder error detected - possibly malformed binary data');
            }
            throw queryError;
        }

        console.log(
            `[Fetch] Successfully fetched fresh data for cursor: ${cursor}`
        );
        return result;
    } catch (error) {
        console.error('Error in Fetch:', error);
        throw error;
    }
}
