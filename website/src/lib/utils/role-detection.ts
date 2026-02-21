import type { UserActivity } from '$lib/services/api-user-activity';
import type { Offering } from '$lib/services/api';

export type UserRole = 'new' | 'tenant' | 'provider';

/**
 * Determines the user's primary role from their activity and offerings.
 *
 * - 'new'      : no contracts AND no offerings
 * - 'provider' : >= 1 offering
 * - 'tenant'   : has contracts but <= 1 offering
 */
export function detectUserRole(
	activity: UserActivity | null,
	myOfferings: Offering[]
): UserRole {
	const hasOfferings = myOfferings.length >= 1;
	const hasContracts = (activity?.rentals_as_requester.length ?? 0) > 0
		|| (activity?.offerings_provided.length ?? 0) > 0;

	if (!hasOfferings && !hasContracts) return 'new';
	if (hasOfferings) return 'provider';
	return 'tenant';
}

/**
 * Counts active rentals (as requester) from activity.
 */
export function countActiveRentals(activity: UserActivity | null): number {
	if (!activity) return 0;
	return activity.rentals_as_requester.filter(
		(c) => c.status === 'active' || c.status === 'provisioned'
	).length;
}

/**
 * Counts contracts expiring within the given number of days (as requester).
 */
export function countExpiringSoon(activity: UserActivity | null, withinDays: number): number {
	if (!activity) return 0;
	const cutoffMs = Date.now() + withinDays * 24 * 60 * 60 * 1000;
	return activity.rentals_as_requester.filter((c) => {
		if (c.status !== 'active' && c.status !== 'provisioned') return false;
		if (!c.end_timestamp_ns) return false;
		const endMs = c.end_timestamp_ns / 1_000_000;
		return endMs > Date.now() && endMs <= cutoffMs;
	}).length;
}

/**
 * Counts active rentals as provider from activity.
 */
export function countActiveRentalsAsProvider(activity: UserActivity | null): number {
	if (!activity) return 0;
	return activity.rentals_as_provider.filter(
		(c) => c.status === 'active' || c.status === 'provisioned'
	).length;
}
