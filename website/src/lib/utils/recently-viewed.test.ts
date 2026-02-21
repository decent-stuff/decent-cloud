import { describe, it, expect, beforeEach } from 'vitest';
import { recordView, getRecentlyViewed, clearRecentlyViewed } from './recently-viewed';

describe('recently-viewed', () => {
	beforeEach(() => {
		clearRecentlyViewed();
	});

	it('returns empty list when nothing viewed', () => {
		expect(getRecentlyViewed()).toEqual([]);
	});

	it('records a new view and returns it first', () => {
		recordView(42);
		expect(getRecentlyViewed()).toEqual([42]);
	});

	it('deduplicates: recording same ID again moves it to front', () => {
		recordView(1);
		recordView(2);
		recordView(1);
		expect(getRecentlyViewed()).toEqual([1, 2]);
	});

	it('keeps most recent first', () => {
		recordView(10);
		recordView(20);
		recordView(30);
		expect(getRecentlyViewed()).toEqual([30, 20, 10]);
	});

	it('enforces max 10 items, dropping oldest', () => {
		for (let i = 1; i <= 11; i++) recordView(i);
		const result = getRecentlyViewed();
		expect(result.length).toBe(10);
		expect(result[0]).toBe(11);
		expect(result).not.toContain(1);
	});

	it('clearRecentlyViewed resets the list', () => {
		recordView(5);
		recordView(6);
		clearRecentlyViewed();
		expect(getRecentlyViewed()).toEqual([]);
	});
});
