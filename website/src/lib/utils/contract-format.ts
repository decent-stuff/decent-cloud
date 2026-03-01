import { bytesToHex, truncatePubkey } from '$lib/utils/identity';
import { Principal } from '@dfinity/principal';

export function formatContractDate(timestamp_ns?: number): string {
	if (!timestamp_ns) return 'N/A';
	const date = new Date(timestamp_ns / 1_000_000);
	return `${date.toLocaleDateString()} ${date.toLocaleTimeString()}`;
}

export function formatContractPrice(amount_e9s: number, currency: string): string {
	return `${(amount_e9s / 1_000_000_000).toFixed(2)} ${currency.toUpperCase()}`;
}

/**
 * Truncates a hash/pubkey for display. Delegates to truncatePubkey for consistency.
 */
export function truncateContractHash(hash: string, visible: number = 6): string {
	return truncatePubkey(hash, visible);
}

/**
 * Convert Ed25519 public key bytes to hex string.
 * Returns the raw public key hex (not a hash) so IC Principal can be derived from it.
 */
export function computePubkey(publicKeyBytes: Uint8Array): string {
	return bytesToHex(publicKeyBytes);
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
 * Uses provisioning_completed_at_ns as start time (when service actually started).
 * Falls back to created_at_ns if never provisioned.
 * For cancelled/completed: uses status_updated_at_ns - start_time
 * For active: uses current time - start_time
 */
export function calculateActualDuration(
	created_at_ns: number,
	status: string,
	status_updated_at_ns?: number,
	provisioning_completed_at_ns?: number
): number {
	// Use provisioning time as start, fall back to created time if never provisioned
	const start_ns = provisioning_completed_at_ns ?? created_at_ns;

	if (status === 'cancelled' || status === 'completed') {
		return status_updated_at_ns ? status_updated_at_ns - start_ns : 0;
	}
	return Date.now() * 1_000_000 - start_ns;
}

/**
 * Format a nanosecond timestamp as a relative time string.
 * E.g., "just now", "3m ago", "2h ago", "5d ago", "never"
 */
export function formatRelativeTime(ns: number | null): string {
	if (ns == null) return 'never';
	const diffMs = Date.now() - ns / 1_000_000;
	if (diffMs < 0) return 'just now';
	const diffS = diffMs / 1000;
	if (diffS < 60) return 'just now';
	const diffMin = diffS / 60;
	if (diffMin < 60) return `${Math.floor(diffMin)}m ago`;
	const diffH = diffMin / 60;
	if (diffH < 24) return `${Math.floor(diffH)}h ago`;
	const diffD = diffH / 24;
	if (diffD < 30) return `${Math.floor(diffD)}d ago`;
	return `${Math.floor(diffD / 30)}mo ago`;
}

/**
 * Format time remaining until a contract's end timestamp.
 * Returns null if no end timestamp or if already expired.
 * urgency: 'critical' (<24h), 'warning' (<7d), 'normal' (>=7d)
 */
export function formatTimeRemaining(
	end_timestamp_ns: number | undefined
): { text: string; urgency: 'critical' | 'warning' | 'normal' } | null {
	if (!end_timestamp_ns) return null;
	const remainingMs = end_timestamp_ns / 1_000_000 - Date.now();
	if (remainingMs <= 0) return null;
	const remainingH = remainingMs / (1000 * 60 * 60);
	if (remainingH < 24) {
		return { text: `${Math.ceil(remainingH)}h left`, urgency: 'critical' };
	}
	const remainingD = remainingH / 24;
	if (remainingD < 7) {
		return { text: `${Math.floor(remainingD)}d left`, urgency: 'warning' };
	}
	return { text: `${Math.floor(remainingD)}d left`, urgency: 'normal' };
}

const PROVISIONING_STATUSES = new Set(['provisioning', 'pending', 'accepted']);
const STUCK_THRESHOLD_MS = 30 * 60 * 1000; // 30 minutes

/**
 * Returns elapsed time string (e.g. "2m ago") if the contract is in a
 * provisioning-like state, otherwise null.
 * Uses status_updated_at_ns when available (transition into current state),
 * falling back to created_at_ns.
 */
export function getProvisioningElapsed(
	status: string,
	created_at_ns: number,
	status_updated_at_ns?: number
): string | null {
	if (!PROVISIONING_STATUSES.has(status.toLowerCase())) return null;
	const ref_ns = status_updated_at_ns ?? created_at_ns;
	return formatRelativeTime(ref_ns);
}

/**
 * Returns true when a contract has been in a provisioning-like state for
 * more than 30 minutes, indicating something may be wrong.
 */
export function isProvisioningStuck(
	status: string,
	created_at_ns: number,
	status_updated_at_ns?: number
): boolean {
	if (!PROVISIONING_STATUSES.has(status.toLowerCase())) return false;
	const ref_ns = status_updated_at_ns ?? created_at_ns;
	const elapsedMs = Date.now() - ref_ns / 1_000_000;
	return elapsedMs > STUCK_THRESHOLD_MS;
}

/**
 * Format duration from nanoseconds to human-readable string.
 */
export function formatDuration(duration_ns: number): string {
	const hours = duration_ns / (1_000_000_000 * 60 * 60);
	if (hours < 1) {
		const minutes = duration_ns / (1_000_000_000 * 60);
		return `${minutes.toFixed(1)}min`;
	}
	if (hours < 24) {
		return `${hours.toFixed(1)}h`;
	}
	const days = hours / 24;
	return `${days.toFixed(1)}d`;
}

export interface ContractForSpending {
	payment_amount_e9s?: number;
	currency?: string;
}

export function calculateSpendingByCurrency(contracts: ContractForSpending[]): Map<string, number> {
	const byCurrency = new Map<string, number>();
	for (const c of contracts) {
		const currency = c.currency?.toUpperCase() || 'USD';
		const amount = (c.payment_amount_e9s ?? 0) / 1e9;
		byCurrency.set(currency, (byCurrency.get(currency) ?? 0) + amount);
	}
	return byCurrency;
}
