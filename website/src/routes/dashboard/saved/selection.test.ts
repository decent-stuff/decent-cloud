import { describe, it, expect } from 'vitest';

type Offering = {
	id?: number;
};

function toggleSelect(selectedIds: Set<number>, offeringId: number): Set<number> {
	const newSet = new Set(selectedIds);
	if (newSet.has(offeringId)) {
		newSet.delete(offeringId);
	} else {
		newSet.add(offeringId);
	}
	return newSet;
}

function toggleSelectAll(offerings: Offering[], allSelected: boolean): Set<number> {
	if (allSelected) {
		return new Set();
	}
	return new Set(offerings.map((o) => o.id).filter((id): id is number => id !== undefined));
}

function isAllSelected(offerings: Offering[], selectedIds: Set<number>): boolean {
	return (
		offerings.length > 0 && offerings.every((o) => o.id !== undefined && selectedIds.has(o.id))
	);
}

function isSomeSelected(selectedIds: Set<number>): boolean {
	return selectedIds.size > 0;
}

function getIdsToRemoveAfterBulk(
	offerings: Offering[],
	idsToRemove: number[],
	failedIds: number[]
): Offering[] {
	return offerings.filter(
		(o) => o.id === undefined || !idsToRemove.includes(o.id) || failedIds.includes(o.id)
	);
}

describe('toggleSelect', () => {
	it('adds an ID that is not selected', () => {
		const result = toggleSelect(new Set([1, 2]), 3);
		expect(result.has(3)).toBe(true);
		expect(result.size).toBe(3);
	});

	it('removes an ID that is already selected', () => {
		const result = toggleSelect(new Set([1, 2, 3]), 2);
		expect(result.has(2)).toBe(false);
		expect(result.size).toBe(2);
	});

	it('returns a new Set (does not mutate the original)', () => {
		const original = new Set([1, 2]);
		const result = toggleSelect(original, 3);
		expect(original.size).toBe(2);
		expect(result).not.toBe(original);
	});

	it('handles empty set by adding the ID', () => {
		const result = toggleSelect(new Set(), 5);
		expect(result.size).toBe(1);
		expect(result.has(5)).toBe(true);
	});
});

describe('toggleSelectAll', () => {
	const offerings: Offering[] = [{ id: 1 }, { id: 2 }, { id: 3 }];

	it('selects all offerings when not all selected', () => {
		const result = toggleSelectAll(offerings, false);
		expect(result.size).toBe(3);
		expect(result.has(1)).toBe(true);
		expect(result.has(2)).toBe(true);
		expect(result.has(3)).toBe(true);
	});

	it('deselects all when already all selected', () => {
		const result = toggleSelectAll(offerings, true);
		expect(result.size).toBe(0);
	});

	it('ignores offerings without id', () => {
		const offeringsWithMissing: Offering[] = [{ id: 1 }, { id: undefined }, { id: 3 }];
		const result = toggleSelectAll(offeringsWithMissing, false);
		expect(result.size).toBe(2);
		expect(result.has(1)).toBe(true);
		expect(result.has(3)).toBe(true);
	});

	it('returns empty set for empty offerings', () => {
		const result = toggleSelectAll([], false);
		expect(result.size).toBe(0);
	});
});

describe('isAllSelected', () => {
	it('returns true when all offerings are selected', () => {
		const offerings: Offering[] = [{ id: 1 }, { id: 2 }, { id: 3 }];
		const selectedIds = new Set([1, 2, 3]);
		expect(isAllSelected(offerings, selectedIds)).toBe(true);
	});

	it('returns false when some offerings are not selected', () => {
		const offerings: Offering[] = [{ id: 1 }, { id: 2 }, { id: 3 }];
		const selectedIds = new Set([1, 2]);
		expect(isAllSelected(offerings, selectedIds)).toBe(false);
	});

	it('returns false when no offerings exist', () => {
		expect(isAllSelected([], new Set())).toBe(false);
	});

	it('returns false when offerings have undefined ids', () => {
		const offerings: Offering[] = [{ id: undefined }];
		expect(isAllSelected(offerings, new Set())).toBe(false);
	});
});

describe('isSomeSelected', () => {
	it('returns true when at least one item is selected', () => {
		expect(isSomeSelected(new Set([1]))).toBe(true);
		expect(isSomeSelected(new Set([1, 2, 3]))).toBe(true);
	});

	it('returns false when nothing is selected', () => {
		expect(isSomeSelected(new Set())).toBe(false);
	});
});

describe('getIdsToRemoveAfterBulk', () => {
	const offerings: Offering[] = [{ id: 1 }, { id: 2 }, { id: 3 }, { id: 4 }];

	it('removes all specified ids when none failed', () => {
		const result = getIdsToRemoveAfterBulk(offerings, [1, 2], []);
		expect(result.length).toBe(2);
		expect(result.find((o) => o.id === 1)).toBeUndefined();
		expect(result.find((o) => o.id === 2)).toBeUndefined();
		expect(result.find((o) => o.id === 3)).toBeDefined();
		expect(result.find((o) => o.id === 4)).toBeDefined();
	});

	it('keeps failed ids in the list', () => {
		const result = getIdsToRemoveAfterBulk(offerings, [1, 2, 3], [2]);
		expect(result.length).toBe(2);
		expect(result.find((o) => o.id === 1)).toBeUndefined();
		expect(result.find((o) => o.id === 2)).toBeDefined();
		expect(result.find((o) => o.id === 3)).toBeUndefined();
		expect(result.find((o) => o.id === 4)).toBeDefined();
	});

	it('keeps offerings with undefined id', () => {
		const offeringsWithUndefined: Offering[] = [{ id: 1 }, { id: undefined }, { id: 3 }];
		const result = getIdsToRemoveAfterBulk(offeringsWithUndefined, [1], []);
		expect(result.length).toBe(2);
		expect(result.find((o) => o.id === undefined)).toBeDefined();
	});

	it('returns all offerings when nothing to remove', () => {
		const result = getIdsToRemoveAfterBulk(offerings, [], []);
		expect(result.length).toBe(4);
	});
});
