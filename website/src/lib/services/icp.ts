import { HttpAgent, Actor, type Identity } from '@dfinity/agent';
// @ts-expect-error - metadata.js is a JavaScript file with proper exports
import { idlFactory as metadataIdl } from '../utils/metadata.js';
import type { Principal } from '@dfinity/principal';

const defaultConfig = {
	networkUrl: 'https://icp-api.io',
	canisterId: 'ggi4a-wyaaa-aaaai-actqq-cai'
};

let agent: HttpAgent | null = null;
let currentIdentity: Identity | null = null;

const MAX_RETRIES = 3;
const RETRY_DELAY = 3000;

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export function getAgent(identity?: Identity | null): HttpAgent {
	if (!agent || currentIdentity !== identity) {
		try {
			if (identity) {
				agent = HttpAgent.createSync({
					host: defaultConfig.networkUrl,
					shouldFetchRootKey: true,
					identity
				});
				currentIdentity = identity;
			} else {
				agent = HttpAgent.createSync({
					host: defaultConfig.networkUrl,
					shouldFetchRootKey: true
				});
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
				canisterId: defaultConfig.canisterId
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
			canisterId
		});

		if (typeof (actor as any)[methodName] !== 'function') {
			throw new Error(`Method "${methodName}" not found on the canister interface.`);
		}

		return await (actor as any)[methodName](...args);
	} catch (error) {
		console.error('Error in updateCanister:', error);
		throw error;
	}
}

type MetadataEntry = [string, { Nat?: bigint | number }];

function parseTokenValueUsdE6(metadata: unknown): number {
	if (!Array.isArray(metadata)) {
		throw new Error('Invalid metadata payload shape');
	}

	const entry = (metadata as MetadataEntry[]).find(([key]) => key === 'ledger:token_value_in_usd_e6');
	if (!entry) {
		throw new Error('Missing ledger:token_value_in_usd_e6 in canister metadata');
	}

	const rawNat = entry[1]?.Nat;
	if (typeof rawNat !== 'bigint' && typeof rawNat !== 'number') {
		throw new Error('ledger:token_value_in_usd_e6 has non-nat value');
	}

	const e6 = Number(rawNat);
	if (!Number.isFinite(e6) || e6 <= 0) {
		throw new Error(`Invalid ledger:token_value_in_usd_e6 value: ${String(rawNat)}`);
	}

	return e6 / 1_000_000;
}

export async function fetchDctPrice(): Promise<number> {
	try {
		const metadata = await fetchMetadata();
		return parseTokenValueUsdE6(metadata);
	} catch (error) {
		console.error('Error fetching DCT price:', error);
		return 0;
	}
}
