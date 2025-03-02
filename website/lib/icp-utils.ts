import { HttpAgent, Actor } from '@dfinity/agent';
import { idlFactory as metadataIdl } from './metadata.js';

const canisterId = 'ggi4a-wyaaa-aaaai-actqq-cai';

// Create agent lazily to avoid initialization issues
let agent: HttpAgent | null = null;

const MAX_RETRIES = 3;
const RETRY_DELAY = 3000; // 3 seconds

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

async function getAgent() {
  if (!agent) {
    try {
      agent = await HttpAgent.create({
        host: 'https://icp-api.io',
      });
    } catch (error) {
      console.error('Failed to initialize HttpAgent');
      throw error;
    }
  }
  return agent;
}

export async function fetchMetadata() {
  let lastError;

  for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
    try {
      const currentAgent = await getAgent();
      await currentAgent.fetchRootKey();

      const canister = Actor.createActor(metadataIdl, {
        agent: currentAgent,
        canisterId,
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
