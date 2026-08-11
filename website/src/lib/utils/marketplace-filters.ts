/**
 * Pure utility functions for marketplace filtering and price display.
 * Extracted for testability — no Svelte/browser dependencies.
 */

export interface OfferingStockable {
	stock_status: string;
}

export interface OfferingOnline {
	provider_online: boolean | undefined;
}

/**
 * Shape required to decide whether an offering is safe to surface as a
 * clickable alternative (e.g. "Similar Offerings"). Both fields are optional
 * because not every caller has the full `Offering` shape (the similar-offerings
 * helper uses a trimmed `SimilarOffering`); the predicate treats absent /
 * `undefined` / `null` values as "no problem" so it composes cleanly.
 */
export interface OfferingRentable {
	provider_online?: boolean | undefined;
	has_critical_flags?: boolean | undefined;
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

/**
 * Returns true when an offering is genuinely rentable right now and therefore
 * safe to link to as an alternative: the provider is NOT confirmed offline
 * (`provider_online !== false`) AND the offering carries no critical flags.
 *
 * Offerings with unknown online status (`undefined`/`null`) are considered
 * rentable — matching the marketplace list and trending-strip "unknown =
 * include" default — so a link never silently drops a live provider whose
 * status simply hasn't been polled.
 *
 * This is the shared liveness guard used wherever we surface a deep link to
 * another offering (e.g. the "Similar Offerings" strip on the detail page);
 * re-use it instead of re-implementing the two conditions inline.
 */
export function isOfferingRentable<T extends OfferingRentable>(offering: T): boolean {
	return offering.provider_online !== false && offering.has_critical_flags !== true;
}
