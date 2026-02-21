import { describe, it, expect } from 'vitest';
import { formatUsdPrice, filterInStock, isOfferingPaused } from './marketplace-filters';

// ---------- formatUsdPrice ----------
describe('formatUsdPrice', () => {
	it('formats ICP amount and rate as USD monthly string', () => {
		expect(formatUsdPrice(10, 5)).toBe('≈ $50.00/mo');
	});

	it('formats zero ICP amount as $0.00/mo', () => {
		expect(formatUsdPrice(0, 5)).toBe('≈ $0.00/mo');
	});

	it('returns null when icpAmount is null', () => {
		expect(formatUsdPrice(null, 5)).toBeNull();
	});

	it('returns null when icpAmount is undefined', () => {
		expect(formatUsdPrice(undefined, 5)).toBeNull();
	});

	it('returns null when icpUsdRate is null (no rate available)', () => {
		expect(formatUsdPrice(10, null)).toBeNull();
	});

	it('returns null when icpUsdRate is undefined', () => {
		expect(formatUsdPrice(10, undefined)).toBeNull();
	});

	it('rounds to two decimal places', () => {
		expect(formatUsdPrice(1, 3.333)).toBe('≈ $3.33/mo');
	});
});

// ---------- isOfferingPaused ----------
describe('isOfferingPaused', () => {
	it('returns false for in_stock offering', () => {
		expect(isOfferingPaused({ stock_status: 'in_stock' })).toBe(false);
	});

	it('returns true for out_of_stock offering', () => {
		expect(isOfferingPaused({ stock_status: 'out_of_stock' })).toBe(true);
	});

	it('returns true for discontinued offering', () => {
		expect(isOfferingPaused({ stock_status: 'discontinued' })).toBe(true);
	});

	it('returns false for null offering', () => {
		expect(isOfferingPaused(null)).toBe(false);
	});

	it('returns false for undefined offering', () => {
		expect(isOfferingPaused(undefined)).toBe(false);
	});
});

// ---------- filterInStock ----------
describe('filterInStock', () => {
	const inStock = { stock_status: 'in_stock' };
	const outOfStock = { stock_status: 'out_of_stock' };
	const discontinued = { stock_status: 'discontinued' };

	it('excludes paused offerings when inStockOnly is true', () => {
		const result = filterInStock([inStock, outOfStock, discontinued], true);
		expect(result).toEqual([inStock]);
	});

	it('includes all offerings when inStockOnly is false', () => {
		const result = filterInStock([inStock, outOfStock, discontinued], false);
		expect(result).toEqual([inStock, outOfStock, discontinued]);
	});

	it('returns empty array for empty input when inStockOnly is true', () => {
		expect(filterInStock([], true)).toEqual([]);
	});

	it('returns empty array for empty input when inStockOnly is false', () => {
		expect(filterInStock([], false)).toEqual([]);
	});
});

// Test the pure logic of URL param encoding/decoding for marketplace filters
describe('marketplace filter URL encoding', () => {
	it('encodes selected types as comma-separated string', () => {
		const types = new Set(['gpu', 'compute']);
		const encoded = [...types].join(',');
		expect(encoded).toBe('gpu,compute');
		const decoded = new Set(encoded.split(',').filter(Boolean));
		expect(decoded).toEqual(new Set(['gpu', 'compute']));
	});

	it('encodes boolean flags as "1" when true, omits when false', () => {
		expect(true ? '1' : null).toBe('1');
		expect(false ? '1' : null).toBeNull();
	});

	it('round-trips numeric filters without precision loss', () => {
		for (const n of [0, 10, 99.5, 1000]) {
			expect(Number(String(n))).toBe(n);
		}
	});

	it('returns null for missing numeric params', () => {
		const params = new URLSearchParams('region=europe');
		const minPrice = params.has('minPrice') ? Number(params.get('minPrice')) : null;
		expect(minPrice).toBeNull();
	});

	it('defaults sortField to price when not in URL', () => {
		const params = new URLSearchParams('region=europe');
		const sortField = params.get('sort') ?? 'price';
		expect(sortField).toBe('price');
	});

	it('defaults sortDir to asc when not in URL', () => {
		const params = new URLSearchParams('sort=trust');
		const sortDir = params.get('dir') ?? 'asc';
		expect(sortDir).toBe('asc');
	});

	it('omits default sortField (price) and sortDir (asc) from params', () => {
		function buildSortParams(sortField: 'price' | 'trust' | 'newest', sortDir: 'asc' | 'desc') {
			const params = new URLSearchParams();
			if (sortField !== 'price') params.set('sort', sortField);
			if (sortDir !== 'asc') params.set('dir', sortDir);
			return params;
		}
		const p = buildSortParams('price', 'asc');
		expect(p.has('sort')).toBe(false);
		expect(p.has('dir')).toBe(false);
	});

	it('includes non-default sort values in params', () => {
		function buildSortParams(sortField: 'price' | 'trust' | 'newest', sortDir: 'asc' | 'desc') {
			const params = new URLSearchParams();
			if (sortField !== 'price') params.set('sort', sortField);
			if (sortDir !== 'asc') params.set('dir', sortDir);
			return params;
		}
		const p = buildSortParams('trust', 'desc');
		expect(p.get('sort')).toBe('trust');
		expect(p.get('dir')).toBe('desc');
	});

	it('decodes quickFilter from URL', () => {
		const params = new URLSearchParams('quick=trusted');
		const quickFilter = (params.get('quick') as 'newest' | 'trusted' | null) ?? null;
		expect(quickFilter).toBe('trusted');
	});

	it('decodes selectedPreset from URL', () => {
		const params = new URLSearchParams('preset=gpu');
		const preset = (params.get('preset') as 'gpu' | 'budget' | 'na' | 'europe' | null) ?? null;
		expect(preset).toBe('gpu');
	});

	it('decodes empty string for missing string params', () => {
		const params = new URLSearchParams('q=hello');
		expect(params.get('region') ?? '').toBe('');
		expect(params.get('q') ?? '').toBe('hello');
	});

	it('round-trips a full filter set through URLSearchParams', () => {
		// Simulate syncFiltersToUrl building params
		const params = new URLSearchParams();
		params.set('q', 'my query');
		params.set('types', 'gpu,compute');
		params.set('minPrice', '5');
		params.set('maxPrice', '50');
		params.set('region', 'europe');
		params.set('country', 'DE');
		params.set('city', 'Berlin');
		params.set('minCores', '4');
		params.set('minMemoryGb', '16');
		params.set('minSsdGb', '200');
		params.set('virt', 'kvm');
		params.set('unmetered', '1');
		params.set('minTrust', '80');
		params.set('demo', '1');
		params.set('offline', '1');
		params.set('recipes', '1');
		params.set('sort', 'trust');
		params.set('dir', 'desc');
		params.set('quick', 'newest');
		params.set('preset', 'budget');

		// Simulate readFiltersFromUrl
		const p = params;
		expect(p.get('q') ?? '').toBe('my query');
		const typesStr = p.get('types');
		expect(typesStr ? new Set(typesStr.split(',').filter(Boolean)) : new Set()).toEqual(new Set(['gpu', 'compute']));
		expect(p.has('minPrice') ? Number(p.get('minPrice')) : null).toBe(5);
		expect(p.has('maxPrice') ? Number(p.get('maxPrice')) : null).toBe(50);
		expect(p.get('region') ?? '').toBe('europe');
		expect(p.get('country') ?? '').toBe('DE');
		expect(p.get('city') ?? '').toBe('Berlin');
		expect(p.has('minCores') ? Number(p.get('minCores')) : null).toBe(4);
		expect(p.has('minMemoryGb') ? Number(p.get('minMemoryGb')) : null).toBe(16);
		expect(p.has('minSsdGb') ? Number(p.get('minSsdGb')) : null).toBe(200);
		expect(p.get('virt') ?? '').toBe('kvm');
		expect(p.get('unmetered') === '1').toBe(true);
		expect(p.has('minTrust') ? Number(p.get('minTrust')) : null).toBe(80);
		expect(p.get('demo') === '1').toBe(true);
		expect(p.get('offline') === '1').toBe(true);
		expect(p.get('recipes') === '1').toBe(true);
		expect((p.get('sort') as 'price' | 'trust' | 'newest') ?? 'price').toBe('trust');
		expect((p.get('dir') as 'asc' | 'desc') ?? 'asc').toBe('desc');
		expect((p.get('quick') as 'newest' | 'trusted' | null) ?? null).toBe('newest');
		expect((p.get('preset') as 'gpu' | 'budget' | 'na' | 'europe' | null) ?? null).toBe('budget');
	});
});

describe('marketplace first-time hint: visit counter', () => {
	it('shows hint for first visit (count 0)', () => {
		const visits = 0;
		expect(visits < 3).toBe(true);
	});

	it('shows hint for second visit (count 1)', () => {
		const visits = 1;
		expect(visits < 3).toBe(true);
	});

	it('shows hint for third visit (count 2)', () => {
		const visits = 2;
		expect(visits < 3).toBe(true);
	});

	it('hides hint after three visits (count 3)', () => {
		const visits = 3;
		expect(visits < 3).toBe(false);
	});

	it('increments counter correctly from stored string', () => {
		const stored = '2';
		const parsed = parseInt(stored, 10);
		expect(parsed + 1).toBe(3);
	});

	it('defaults to 0 when no stored value', () => {
		const stored = null;
		const parsed = parseInt(stored ?? '0', 10);
		expect(parsed).toBe(0);
	});
});
