import type { UserActivity } from '$lib/services/api-user-activity';

type Contract = UserActivity['rentals_as_requester'][number];

const SEVEN_DAYS_MS = 7 * 24 * 60 * 60 * 1000;
const TWENTY_FOUR_HOURS_MS = 24 * 60 * 60 * 1000;

/**
 * Returns active/provisioned contracts expiring within 7 days relative to nowMs.
 */
export function getExpiringContracts(contracts: Contract[], nowMs: number): Contract[] {
	const cutoffMs = nowMs + SEVEN_DAYS_MS;
	return contracts.filter((c) => {
		if (c.status !== 'active' && c.status !== 'provisioned') return false;
		if (!c.end_timestamp_ns) return false;
		const endMs = c.end_timestamp_ns / 1_000_000;
		return endMs > nowMs && endMs <= cutoffMs;
	});
}

/**
 * Returns true if the contract expires within 24 hours of nowMs.
 */
export function isUrgent(contract: Contract, nowMs: number): boolean {
	if (!contract.end_timestamp_ns) return false;
	const endMs = contract.end_timestamp_ns / 1_000_000;
	return endMs - nowMs <= TWENTY_FOUR_HOURS_MS;
}

/**
 * Formats the expiry banner message based on count and urgency.
 */
export function getExpiryBannerText(count: number, hasUrgent: boolean): string {
	const noun = count === 1 ? 'contract' : 'contracts';
	if (hasUrgent) {
		return `${count} ${noun} expiring soon — action required within 24h`;
	}
	return `${count} ${noun} expiring within 7 days`;
}
