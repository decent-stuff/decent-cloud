const STORAGE_KEY = 'dc-recently-viewed-offerings';
const MAX_ITEMS = 10;

export function recordView(id: number): void {
	if (typeof window === 'undefined') return;
	const current = getRecentlyViewed();
	const filtered = current.filter(i => i !== id);
	const updated = [id, ...filtered].slice(0, MAX_ITEMS);
	localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
}

export function getRecentlyViewed(): number[] {
	if (typeof window === 'undefined') return [];
	try {
		return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]');
	} catch {
		return [];
	}
}

export function clearRecentlyViewed(): void {
	if (typeof window === 'undefined') return;
	localStorage.removeItem(STORAGE_KEY);
}
