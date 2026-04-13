import { describe, it, expect, beforeEach, vi } from 'vitest';

const STORAGE_KEY = 'dc-theme';

function createMockThemeStore() {
	const listeners: Array<(value: 'dark' | 'light') => void> = [];
	let storeValue: 'dark' | 'light';

	function applyTheme(theme: 'dark' | 'light') {
		document.documentElement.setAttribute('data-theme', theme);
	}

	function init() {
		const stored = localStorage.getItem(STORAGE_KEY);
		if (stored === 'light' || stored === 'dark') {
			storeValue = stored;
		} else {
			storeValue = window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
		}
		applyTheme(storeValue);
	}

	init();

	return {
		subscribe(fn: (value: 'dark' | 'light') => void) {
			listeners.push(fn);
			fn(storeValue);
			return () => {
				const idx = listeners.indexOf(fn);
				if (idx >= 0) listeners.splice(idx, 1);
			};
		},
		get current() {
			return storeValue;
		},
		toggle() {
			storeValue = storeValue === 'dark' ? 'light' : 'dark';
			localStorage.setItem(STORAGE_KEY, storeValue);
			applyTheme(storeValue);
			for (const fn of listeners) fn(storeValue);
		},
		set(theme: 'dark' | 'light') {
			storeValue = theme;
			localStorage.setItem(STORAGE_KEY, theme);
			applyTheme(theme);
			for (const fn of listeners) fn(storeValue);
		}
	};
}

describe('theme store: initial load', () => {
	beforeEach(() => {
		localStorage.clear();
		document.documentElement.removeAttribute('data-theme');
	});

	it('defaults to dark when no stored preference and system is dark', () => {
		window.matchMedia = vi.fn().mockReturnValue({ matches: false, addEventListener: vi.fn() });
		const store = createMockThemeStore();
		expect(store.current).toBe('dark');
	});

	it('defaults to light when system prefers light', () => {
		window.matchMedia = vi.fn().mockReturnValue({ matches: true, addEventListener: vi.fn() });
		const store = createMockThemeStore();
		expect(store.current).toBe('light');
	});

	it('restores stored theme from localStorage', () => {
		localStorage.setItem(STORAGE_KEY, 'light');
		const store = createMockThemeStore();
		expect(store.current).toBe('light');
	});

	it('applies data-theme attribute on init', () => {
		localStorage.setItem(STORAGE_KEY, 'light');
		createMockThemeStore();
		expect(document.documentElement.getAttribute('data-theme')).toBe('light');
	});
});

describe('theme store: toggle', () => {
	beforeEach(() => {
		localStorage.clear();
		document.documentElement.removeAttribute('data-theme');
		window.matchMedia = vi.fn().mockReturnValue({ matches: false, addEventListener: vi.fn() });
	});

	it('toggles from dark to light', () => {
		const store = createMockThemeStore();
		expect(store.current).toBe('dark');
		store.toggle();
		expect(store.current).toBe('light');
	});

	it('toggles from light to dark', () => {
		const store = createMockThemeStore();
		store.toggle();
		store.toggle();
		expect(store.current).toBe('dark');
	});

	it('persists toggle to localStorage', () => {
		const store = createMockThemeStore();
		store.toggle();
		expect(localStorage.getItem(STORAGE_KEY)).toBe('light');
	});

	it('updates data-theme attribute on toggle', () => {
		const store = createMockThemeStore();
		store.toggle();
		expect(document.documentElement.getAttribute('data-theme')).toBe('light');
		store.toggle();
		expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
	});

	it('notifies subscribers on toggle', () => {
		const store = createMockThemeStore();
		const values: string[] = [];
		store.subscribe((v) => values.push(v));
		store.toggle();
		expect(values).toEqual(['dark', 'light']);
	});
});

describe('theme store: set', () => {
	beforeEach(() => {
		localStorage.clear();
		document.documentElement.removeAttribute('data-theme');
		window.matchMedia = vi.fn().mockReturnValue({ matches: false, addEventListener: vi.fn() });
	});

	it('sets theme explicitly', () => {
		const store = createMockThemeStore();
		store.set('light');
		expect(store.current).toBe('light');
		expect(localStorage.getItem(STORAGE_KEY)).toBe('light');
		expect(document.documentElement.getAttribute('data-theme')).toBe('light');
	});

	it('overwrites previous toggle', () => {
		const store = createMockThemeStore();
		store.toggle();
		store.set('dark');
		expect(store.current).toBe('dark');
		expect(localStorage.getItem(STORAGE_KEY)).toBe('dark');
	});
});

describe('theme store: subscribe', () => {
	beforeEach(() => {
		localStorage.clear();
		document.documentElement.removeAttribute('data-theme');
		window.matchMedia = vi.fn().mockReturnValue({ matches: false, addEventListener: vi.fn() });
	});

	it('receives current value on subscribe', () => {
		const store = createMockThemeStore();
		let received: string | undefined;
		store.subscribe((v) => { received = v; });
		expect(received).toBe('dark');
	});

	it('unsubscribe stops updates', () => {
		const store = createMockThemeStore();
		const values: string[] = [];
		const unsub = store.subscribe((v) => values.push(v));
		unsub();
		store.toggle();
		expect(values).toEqual(['dark']);
	});
});
