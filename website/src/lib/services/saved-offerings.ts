/**
 * Toggle an offering ID in a saved-set.
 * Returns a new Set with the ID added (if absent) or removed (if present).
 * Pure function - never mutates the input.
 */
export function toggleSavedId(ids: Set<number>, targetId: number): Set<number> {
	const next = new Set(ids);
	if (next.has(targetId)) {
		next.delete(targetId);
	} else {
		next.add(targetId);
	}
	return next;
}
