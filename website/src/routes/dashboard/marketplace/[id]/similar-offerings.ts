export type SimilarOffering = {
	id?: number;
	product_type?: string;
	currency?: string;
};

export function filterSimilarOfferings<T extends SimilarOffering>(
	allOfferings: T[],
	mainOffering: SimilarOffering,
	maxResults: number = 4
): T[] {
	const mainProductType = (mainOffering.product_type ?? '').toLowerCase();
	const mainCurrency = (mainOffering.currency ?? '').toUpperCase();

	return allOfferings
		.filter((o) => (o.product_type ?? '').toLowerCase() === mainProductType)
		.filter((o) => o.id !== mainOffering.id)
		.filter((o) => (o.currency ?? '').toUpperCase() === mainCurrency)
		.slice(0, maxResults);
}
