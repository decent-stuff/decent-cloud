import { sha256 } from '@noble/hashes/sha2';
import { hexEncode } from '$lib/services/api';

export function formatContractDate(timestamp_ns?: number): string {
	if (!timestamp_ns) return 'N/A';
	const date = new Date(timestamp_ns / 1_000_000);
	return `${date.toLocaleDateString()} ${date.toLocaleTimeString()}`;
}

export function formatContractPrice(amount_e9s: number): string {
	return `${(amount_e9s / 1_000_000_000).toFixed(2)} ICP`;
}

export function truncateContractHash(hash: string, visible: number = 6): string {
	if (!hash) return '';
	if (hash.length <= visible * 2) {
		return hash;
	}
	return `${hash.slice(0, visible)}...${hash.slice(-visible)}`;
}

export function computePubkeyHash(publicKeyBytes: Uint8Array): string {
	const hash = sha256(publicKeyBytes);
	return hexEncode(hash);
}
