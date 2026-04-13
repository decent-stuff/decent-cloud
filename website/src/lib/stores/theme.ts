import { writable, get } from 'svelte/store';
import { browser } from '$app/environment';

export type Theme = 'dark' | 'light';

const STORAGE_KEY = 'dc-theme';

function getSystemPreference(): Theme {
	if (!browser) return 'dark';
	return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

function getStoredTheme(): Theme | null {
	if (!browser) return null;
	const stored = localStorage.getItem(STORAGE_KEY);
	if (stored === 'light' || stored === 'dark') return stored;
	return null;
}

function applyTheme(theme: Theme) {
	if (!browser) return;
	document.documentElement.setAttribute('data-theme', theme);
}

function createThemeStore() {
	const initial: Theme = getStoredTheme() ?? getSystemPreference();
	const store = writable<Theme>(initial);

	applyTheme(initial);

	if (browser) {
		const mql = window.matchMedia('(prefers-color-scheme: light)');
		mql.addEventListener('change', (e) => {
			if (!localStorage.getItem(STORAGE_KEY)) {
				const next = e.matches ? 'light' : 'dark';
				store.set(next);
				applyTheme(next);
			}
		});
	}

	return {
		subscribe: store.subscribe,
		get current() {
			return get(store);
		},
		toggle() {
			const next: Theme = get(store) === 'dark' ? 'light' : 'dark';
			localStorage.setItem(STORAGE_KEY, next);
			store.set(next);
			applyTheme(next);
		},
		set(theme: Theme) {
			localStorage.setItem(STORAGE_KEY, theme);
			store.set(theme);
			applyTheme(theme);
		}
	};
}

export const themeStore = createThemeStore();
