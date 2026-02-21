/**
 * Pure utility functions for marketplace filtering and price display.
 * Extracted for testability — no Svelte/browser dependencies.
 */

export interface OfferingStockable {
	stock_status: string;
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
