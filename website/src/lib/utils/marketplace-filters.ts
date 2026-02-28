/**
 * Pure utility functions for marketplace filtering and price display.
 * Extracted for testability — no Svelte/browser dependencies.
 */

export interface OfferingStockable {
	stock_status: string;
}

export interface OfferingExample {
	is_example: boolean;
}

export interface OfferingOnline {
	provider_online: boolean | undefined;
}

/**
 * Filters offerings by demo status.
 * When includeDemo is false, excludes offerings where is_example is true.
 */
export function filterDemoOfferings<T extends OfferingExample>(
	offerings: T[],
	includeDemo: boolean
): T[] {
	if (includeDemo) return offerings;
	return offerings.filter((o) => !o.is_example);
}

/**
 * Formats an ICP amount as a USD equivalent monthly price string.
 * Returns null if either argument is null/undefined (no rate available or no price).
 */
export function formatUsdPrice(
	icpAmount: number | null | undefined,
	icpUsdRate: number | null | undefined
): string | null {
	if (icpAmount == null || icpUsdRate == null) return null;
	return `≈ $${(icpAmount * icpUsdRate).toFixed(2)}/mo`;
}

/**
 * Returns true when an offering is paused (stock_status !== 'in_stock').
 * Treats null/undefined stock_status as not paused.
 */
export function isOfferingPaused(offering: OfferingStockable | null | undefined): boolean {
	if (!offering) return false;
	return offering.stock_status !== 'in_stock';
}

/**
 * Filters offerings by stock availability.
 * When inStockOnly is true, excludes offerings where stock_status !== 'in_stock'.
 */
export function filterInStock<T extends OfferingStockable>(
	offerings: T[],
	inStockOnly: boolean
): T[] {
	if (!inStockOnly) return offerings;
	return offerings.filter((o) => o.stock_status === 'in_stock');
}

/**
 * Filters offerings by provider online status.
 * When includeOffline is false (default), excludes offerings where provider_online is false.
 * Offerings with undefined provider_online (unknown status) are included by default.
 */
export function filterOfflineOfferings<T extends OfferingOnline>(
	offerings: T[],
	includeOffline: boolean
): T[] {
	if (includeOffline) return offerings;
	return offerings.filter((o) => o.provider_online !== false);
}
