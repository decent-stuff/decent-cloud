import { hexEncode } from '$lib/services/api';
import { Principal } from '@dfinity/principal';

export function formatContractDate(timestamp_ns?: number): string {
	if (!timestamp_ns) return 'N/A';
	const date = new Date(timestamp_ns / 1_000_000);
	return `${date.toLocaleDateString()} ${date.toLocaleTimeString()}`;
}

export function formatContractPrice(amount_e9s: number, currency: string): string {
	return `${(amount_e9s / 1_000_000_000).toFixed(2)} ${currency.toUpperCase()}`;
}

export function truncateContractHash(hash: string, visible: number = 6): string {
	if (!hash) return '';
	if (hash.length <= visible * 2) {
		return hash;
	}
	return `${hash.slice(0, visible)}...${hash.slice(-visible)}`;
}

/**
 * Convert Ed25519 public key bytes to hex string.
 * Returns the raw public key hex (not a hash) so IC Principal can be derived from it.
 */
export function computePubkey(publicKeyBytes: Uint8Array): string {
	return hexEncode(publicKeyBytes);
}

/**
 * Derives an IC self-authenticating Principal from an Ed25519 public key.
 * The public key must be DER-encoded for the IC to recognize it.
 */
export function derivePrincipalFromPubkey(publicKeyBytes: Uint8Array): Principal {
	// Ed25519 DER prefix for public keys (as per RFC 8410)
	const DER_PREFIX = new Uint8Array([
		0x30, 0x2a, // SEQUENCE of 42 bytes
		0x30, 0x05, // SEQUENCE of 5 bytes
		0x06, 0x03, 0x2b, 0x65, 0x70, // OID 1.3.101.112 (Ed25519)
		0x03, 0x21, 0x00 // BIT STRING of 33 bytes (0x00 + 32-byte key)
	]);

	// Combine DER prefix with the raw 32-byte public key
	const derEncodedKey = new Uint8Array(DER_PREFIX.length + publicKeyBytes.length);
	derEncodedKey.set(DER_PREFIX);
	derEncodedKey.set(publicKeyBytes, DER_PREFIX.length);

	// Create self-authenticating principal from DER-encoded key
	return Principal.selfAuthenticating(derEncodedKey);
}

/**
 * Calculate actual runtime duration of a contract in nanoseconds.
 * For cancelled/completed: uses status_updated_at_ns - created_at_ns
 * For active: uses current time - created_at_ns
 */
export function calculateActualDuration(
	created_at_ns: number,
	status: string,
	status_updated_at_ns?: number
): number {
	if (status === 'cancelled' || status === 'completed') {
		return status_updated_at_ns ? status_updated_at_ns - created_at_ns : 0;
	}
	return Date.now() * 1_000_000 - created_at_ns;
}

/**
 * Format duration from nanoseconds to human-readable string.
 */
export function formatDuration(duration_ns: number): string {
	const hours = duration_ns / (1_000_000_000 * 60 * 60);
	if (hours < 1) {
		const minutes = duration_ns / (1_000_000_000 * 60);
		return `${minutes.toFixed(1)}m`;
	}
	if (hours < 24) {
		return `${hours.toFixed(1)}h`;
	}
	const days = hours / 24;
	return `${days.toFixed(1)}d`;
}
