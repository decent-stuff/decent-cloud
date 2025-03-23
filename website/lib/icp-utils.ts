import { HttpAgent, Actor, Identity } from '@dfinity/agent';
import { idlFactory as metadataIdl } from './metadata.js';
import { Principal } from '@dfinity/principal';

const defaultConfig = {
  networkUrl: 'https://icp-api.io',
  canisterId: 'ggi4a-wyaaa-aaaai-actqq-cai',
};

// Singleton agent instance
let agent: HttpAgent | null = null;
let currentIdentity: Identity | null = null;

const MAX_RETRIES = 3;
const RETRY_DELAY = 3000; // 3 seconds

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

export function getAgent(identity?: Identity | null): HttpAgent {
  if (!agent || currentIdentity !== identity) {
    try {
      if (identity) {
        agent = HttpAgent.createSync({
          host: defaultConfig.networkUrl,
          shouldFetchRootKey: true,
          identity: identity,
        });
        console.log('Agent created with identity:', identity.getPrincipal().toString());
        currentIdentity = identity;
      } else {
        agent = HttpAgent.createSync({
          host: defaultConfig.networkUrl,
          shouldFetchRootKey: true,
        });
        console.log('Agent created without identity');
      }
    } catch (error) {
      console.error(`Failed to initialize ${identity ? 'authenticated' : 'anonymous'} HttpAgent`);
      throw error;
    }
  }
  return agent;
}

export async function fetchMetadata() {
  let lastError;

  for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
    try {
      const currentAgent = getAgent();
      await currentAgent.fetchRootKey();

      const canister = Actor.createActor(metadataIdl, {
        agent: currentAgent,
        canisterId: defaultConfig.canisterId,
      });

      const metadata = await canister.metadata();
      return metadata;
    } catch (error) {
      lastError = error;

      if (attempt < MAX_RETRIES - 1) {
        const delay = RETRY_DELAY * Math.pow(2, attempt);
        await sleep(delay);
      }
    }
  }

  throw lastError;
}

interface CanisterCallOptions {
  canisterId?: Principal;
}

/**
 * Call an update canister method (authenticated call)
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

    const actor = Actor.createActor(metadataIdl, {
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
