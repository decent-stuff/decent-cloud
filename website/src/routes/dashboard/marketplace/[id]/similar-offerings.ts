import { isOfferingRentable } from '$lib/utils/marketplace-filters';

export type SimilarOffering = {
	id?: number;
	product_type?: string;
	currency?: string;
	provider_online?: boolean;
	has_critical_flags?: boolean;
};

/**
 * Pick rentable alternatives to the offering being viewed. Mirrors the
 * marketplace list's liveness rules via the shared {@link isOfferingRentable}
 * guard so a "Similar Offerings" link never routes a buyer to a dead/offline
 * detail page (regression: the 1628 page used to surface 956/1206/957/1207 —
 * all `provider_online:false` + `has_critical_flags:true` — as "similar").
 */
export function filterSimilarOfferings<T extends SimilarOffering>(
	allOfferings: T[],
	mainOffering: SimilarOffering,
	maxResults: number = 4
): T[] {
	const mainProductType = (mainOffering.product_type ?? '').toLowerCase();
	const mainCurrency = (mainOffering.currency ?? '').toUpperCase();

	return allOfferings
		.filter(isOfferingRentable)
		.filter((o) => (o.product_type ?? '').toLowerCase() === mainProductType)
		.filter((o) => o.id !== mainOffering.id)
		.filter((o) => (o.currency ?? '').toUpperCase() === mainCurrency)
		.slice(0, maxResults);
}
