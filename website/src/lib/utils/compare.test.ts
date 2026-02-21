import { describe, it, expect } from 'vitest';
import { addToComparison, removeFromComparison, COMPARE_MAX, COMPARE_MAX_ERROR } from './compare';

describe('addToComparison', () => {
	it('adds a new ID to an empty set', () => {
		const result = addToComparison(new Set(), 1);
		expect(result).toEqual(new Set([1]));
	});

	it('adds up to COMPARE_MAX IDs', () => {
		let ids = new Set<number>();
		for (let i = 1; i <= COMPARE_MAX; i++) {
			ids = addToComparison(ids, i);
		}
		expect(ids.size).toBe(COMPARE_MAX);
	});

	it('throws when adding beyond COMPARE_MAX', () => {
		let ids = new Set<number>();
		for (let i = 1; i <= COMPARE_MAX; i++) {
			ids = addToComparison(ids, i);
		}
		expect(() => addToComparison(ids, COMPARE_MAX + 1)).toThrow(COMPARE_MAX_ERROR);
	});

	it('does not mutate the original set', () => {
		const original = new Set([1]);
		addToComparison(original, 2);
		expect(original.size).toBe(1);
	});

	it('is idempotent when adding an already-present ID', () => {
		const ids = new Set([1, 2]);
		const result = addToComparison(ids, 1);
		expect(result.size).toBe(2);
	});

	it('does not throw when re-adding an ID that is already present in a full set', () => {
		let ids = new Set<number>();
		for (let i = 1; i <= COMPARE_MAX; i++) {
			ids = addToComparison(ids, i);
		}
		// Re-adding an existing ID must not throw even when full
		expect(() => addToComparison(ids, 1)).not.toThrow();
	});
});

describe('removeFromComparison', () => {
	it('removes an existing ID', () => {
		const result = removeFromComparison(new Set([1, 2, 3]), 2);
		expect(result).toEqual(new Set([1, 3]));
	});

	it('returns an empty set when removing the last ID', () => {
		const result = removeFromComparison(new Set([5]), 5);
		expect(result.size).toBe(0);
	});

	it('is a no-op when the ID is not present', () => {
		const ids = new Set([1, 2]);
		const result = removeFromComparison(ids, 99);
		expect(result).toEqual(new Set([1, 2]));
	});

	it('does not mutate the original set', () => {
		const original = new Set([1, 2, 3]);
		removeFromComparison(original, 2);
		expect(original.size).toBe(3);
	});
});
