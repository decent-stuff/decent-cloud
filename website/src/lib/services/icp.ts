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

interface TokenMetrics {
	price?: number;
	volume_24h?: number;
	total_supply?: number;
	market_cap?: number;
	tvl?: number;
	updated_at?: string;
	price_change_24h?: number | null;
}

interface KongSwapTokenRaw {
	metrics?: {
		price?: number | string;
		volume_24h?: number | string;
		total_supply?: number | string;
		market_cap?: number | string;
		tvl?: number | string;
		updated_at?: string;
		price_change_24h?: number | string | null;
	};
}

interface TokenResponse {
	items: KongSwapTokenRaw[];
}

const parseMetricValue = (value: number | string | undefined | null): number => {
	if (!value) return 0;
	if (typeof value === 'number') return value;
	if (typeof value === 'string') return parseFloat(value.replaceAll(',', '')) || 0;
	return 0;
};

export async function fetchDctPrice(): Promise<number> {
	try {
		const response = await fetch('https://api.kongswap.io/api/tokens/by_canister', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({
				canister_ids: [defaultConfig.canisterId],
				page: 1,
				limit: 1
			})
		});

		if (!response.ok) {
			throw new Error(`HTTP error! status: ${response.status}`);
		}

		const data = (await response.json()) as TokenResponse;

		if (!data.items || !Array.isArray(data.items) || data.items.length === 0) {
			throw new Error('No token data returned from KongSwap API');
		}

		return parseMetricValue(data.items[0]?.metrics?.price);
	} catch (error) {
		console.error('Error fetching DCT price:', error);
		return 0;
	}
}
