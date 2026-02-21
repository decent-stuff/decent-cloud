export const COMPARE_MAX = 3;
export const COMPARE_MAX_ERROR = `Maximum ${COMPARE_MAX} offerings can be compared`;

/**
 * Add an offering ID to the comparison set.
 * Returns a new Set with the ID added.
 * Throws if the set is already at COMPARE_MAX capacity.
 */
export function addToComparison(ids: Set<number>, newId: number): Set<number> {
	if (ids.has(newId)) return new Set(ids);
	if (ids.size >= COMPARE_MAX) throw new Error(COMPARE_MAX_ERROR);
	return new Set([...ids, newId]);
}

/**
 * Remove an offering ID from the comparison set.
 * Returns a new Set with the ID removed.
 */
export function removeFromComparison(ids: Set<number>, id: number): Set<number> {
	const next = new Set(ids);
	next.delete(id);
	return next;
}
