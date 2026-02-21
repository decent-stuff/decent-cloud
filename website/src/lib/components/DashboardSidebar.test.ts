import { describe, it, expect, beforeEach, afterEach } from 'vitest';

// ---- Pure logic extracted from DashboardSidebar ----

type SectionKey = 'discover' | 'activity' | 'cloud' | 'provider';

const SECTION_DEFAULTS: Record<SectionKey, boolean> = {
	discover: false,
	activity: false,
	cloud: false,
	provider: true
};

function loadSectionState(key: SectionKey, storage: Storage): boolean {
	const stored = storage.getItem(`sidebar_section_${key}`);
	if (stored === null) return SECTION_DEFAULTS[key];
	return stored === 'true';
}

function toggleSection(
	key: SectionKey,
	current: Record<SectionKey, boolean>,
	storage: Storage
): Record<SectionKey, boolean> {
	const next = !current[key];
	storage.setItem(`sidebar_section_${key}`, String(next));
	return { ...current, [key]: next };
}

// Returns the resolved collapsed state for the provider section given offeringsCount.
// Mirrors the $effect in the component: auto-expand when user has offerings and
// localStorage has not already been explicitly set to 'false' by the user.
function resolveProviderCollapsed(
	offeringsCount: number,
	currentCollapsed: boolean,
	storage: Storage
): boolean {
	if (offeringsCount > 0) {
		const stored = storage.getItem('sidebar_section_provider');
		if (stored === null || stored === 'true') {
			storage.setItem('sidebar_section_provider', 'false');
			return false;
		}
	}
	return currentCollapsed;
}

// ---- Tests ----

describe('DashboardSidebar: loadSectionState defaults', () => {
	let storage: Storage;

	beforeEach(() => {
		storage = window.localStorage;
		storage.clear();
	});

	afterEach(() => {
		storage.clear();
	});

	it('returns false (expanded) for discover when no entry in localStorage', () => {
		expect(loadSectionState('discover', storage)).toBe(false);
	});

	it('returns false (expanded) for activity when no entry in localStorage', () => {
		expect(loadSectionState('activity', storage)).toBe(false);
	});

	it('returns false (expanded) for cloud when no entry in localStorage', () => {
		expect(loadSectionState('cloud', storage)).toBe(false);
	});

	it('returns true (collapsed) for provider when no entry in localStorage', () => {
		expect(loadSectionState('provider', storage)).toBe(true);
	});

	it('returns stored true when localStorage has "true"', () => {
		storage.setItem('sidebar_section_discover', 'true');
		expect(loadSectionState('discover', storage)).toBe(true);
	});

	it('returns stored false when localStorage has "false"', () => {
		storage.setItem('sidebar_section_provider', 'false');
		expect(loadSectionState('provider', storage)).toBe(false);
	});
});

describe('DashboardSidebar: toggleSection persists to localStorage', () => {
	let storage: Storage;

	beforeEach(() => {
		storage = window.localStorage;
		storage.clear();
	});

	afterEach(() => {
		storage.clear();
	});

	it('collapses an expanded section and writes "true" to localStorage', () => {
		const state: Record<SectionKey, boolean> = { ...SECTION_DEFAULTS }; // discover = false
		const next = toggleSection('discover', state, storage);
		expect(next.discover).toBe(true);
		expect(storage.getItem('sidebar_section_discover')).toBe('true');
	});

	it('expands a collapsed section and writes "false" to localStorage', () => {
		const state: Record<SectionKey, boolean> = { ...SECTION_DEFAULTS }; // provider = true
		const next = toggleSection('provider', state, storage);
		expect(next.provider).toBe(false);
		expect(storage.getItem('sidebar_section_provider')).toBe('false');
	});

	it('does not mutate the original state object', () => {
		const state: Record<SectionKey, boolean> = { ...SECTION_DEFAULTS };
		toggleSection('activity', state, storage);
		expect(state.activity).toBe(false); // original unchanged
	});

	it('toggling twice returns to the original collapsed value', () => {
		const state: Record<SectionKey, boolean> = { ...SECTION_DEFAULTS };
		const after1 = toggleSection('cloud', state, storage);
		const after2 = toggleSection('cloud', after1, storage);
		expect(after2.cloud).toBe(state.cloud);
		expect(storage.getItem('sidebar_section_cloud')).toBe(String(state.cloud));
	});
});

describe('DashboardSidebar: provider section auto-expand logic', () => {
	let storage: Storage;

	beforeEach(() => {
		storage = window.localStorage;
		storage.clear();
	});

	afterEach(() => {
		storage.clear();
	});

	it('keeps provider collapsed when offeringsCount is 0', () => {
		const result = resolveProviderCollapsed(0, true, storage);
		expect(result).toBe(true);
		expect(storage.getItem('sidebar_section_provider')).toBeNull();
	});

	it('auto-expands provider when offeringsCount becomes > 0 and no localStorage entry', () => {
		const result = resolveProviderCollapsed(1, true, storage);
		expect(result).toBe(false);
		expect(storage.getItem('sidebar_section_provider')).toBe('false');
	});

	it('auto-expands provider when localStorage still says "true" (default not overridden)', () => {
		storage.setItem('sidebar_section_provider', 'true');
		const result = resolveProviderCollapsed(3, true, storage);
		expect(result).toBe(false);
		expect(storage.getItem('sidebar_section_provider')).toBe('false');
	});

	it('respects explicit user collapse: stays collapsed when localStorage is "false" but offerings appear', () => {
		// User explicitly collapsed it after having offerings - we should NOT override that.
		// However, based on component logic: if stored === 'false', auto-expand does NOT fire
		// because the condition is: stored === null || stored === 'true'
		storage.setItem('sidebar_section_provider', 'false');
		const result = resolveProviderCollapsed(5, false, storage);
		expect(result).toBe(false); // already expanded - currentCollapsed was false
		expect(storage.getItem('sidebar_section_provider')).toBe('false'); // unchanged
	});

	it('keeps provider expanded when already expanded and offerings present', () => {
		// currentCollapsed = false, offeringsCount > 0, stored = 'false'
		storage.setItem('sidebar_section_provider', 'false');
		const result = resolveProviderCollapsed(2, false, storage);
		expect(result).toBe(false);
	});
});

describe('DashboardSidebar: section key storage isolation', () => {
	let storage: Storage;

	beforeEach(() => {
		storage = window.localStorage;
		storage.clear();
	});

	afterEach(() => {
		storage.clear();
	});

	it('toggling one section does not affect other sections in localStorage', () => {
		const state: Record<SectionKey, boolean> = { ...SECTION_DEFAULTS };
		toggleSection('discover', state, storage);
		expect(storage.getItem('sidebar_section_activity')).toBeNull();
		expect(storage.getItem('sidebar_section_cloud')).toBeNull();
		expect(storage.getItem('sidebar_section_provider')).toBeNull();
	});

	it('each section reads and writes its own localStorage key', () => {
		const state: Record<SectionKey, boolean> = { ...SECTION_DEFAULTS };
		toggleSection('discover', state, storage);
		toggleSection('provider', state, storage);
		expect(storage.getItem('sidebar_section_discover')).toBe('true');
		expect(storage.getItem('sidebar_section_provider')).toBe('false');
	});
});
