import { describe, expect, it, vi } from 'vitest';
import {
	normalizeCompareIds,
	buildComparePath,
	copyCompareShareUrl,
} from './compare-share';

describe('normalizeCompareIds', () => {
	it('keeps only positive integer IDs, deduplicated and capped', () => {
		expect(normalizeCompareIds(' 3 , 2, 2, 1, 4 ')).toEqual([3, 2, 1]);
	});

	it('rejects parseInt-style partial numbers and invalid tokens', () => {
		expect(normalizeCompareIds('2abc,03,0,-1,11.1,4')).toEqual([3, 4]);
	});
});

describe('buildComparePath', () => {
	it('builds a clean compare path with canonical ids', () => {
		expect(buildComparePath([2, 2, 1, 4])).toBe('/dashboard/marketplace/compare?ids=2,1,4');
	});
});

describe('copyCompareShareUrl', () => {
	it('copies canonical absolute URL to clipboard', async () => {
		const writeText = vi.fn().mockResolvedValue(undefined);

		const copied = await copyCompareShareUrl({
			ids: [2, 2, 1, 4],
			origin: 'https://dev.decent-cloud.org',
			clipboard: { writeText },
		});

		expect(copied).toBe('https://dev.decent-cloud.org/dashboard/marketplace/compare?ids=2,1,4');
		expect(writeText).toHaveBeenCalledWith(copied);
	});

	it('fails loudly when clipboard write fails', async () => {
		const writeText = vi.fn().mockRejectedValue(new Error('clipboard denied'));

		await expect(
			copyCompareShareUrl({
				ids: [1, 2],
				origin: 'https://dev.decent-cloud.org',
				clipboard: { writeText },
			})
		).rejects.toThrow('clipboard denied');
	});
});
