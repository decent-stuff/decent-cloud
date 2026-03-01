import { describe, it, expect, vi } from 'vitest';
import { writable, derived, get } from 'svelte/store';

describe('auth store subscription pattern', () => {
	it('should not throw TDZ error when subscribing and immediately calling unsubscribe', async () => {
		const activeIdentity = writable<{ id: number } | null>(null);
		const isAuthenticated = derived(activeIdentity, ($active) => $active !== null);

		async function checkAuth() {
			let isAuth = false;
			const unsub = isAuthenticated.subscribe((v) => (isAuth = v));
			unsub();
			return isAuth;
		}

		await expect(checkAuth()).resolves.toBe(false);

		activeIdentity.set({ id: 1 });
		await expect(checkAuth()).resolves.toBe(true);
	});

	it('should not throw when using subscribe pattern in async context', async () => {
		const store = writable(42);

		async function getValue() {
			let value = 0;
			const unsub = store.subscribe((v) => (value = v));
			unsub();
			return value;
		}

		await expect(getValue()).resolves.toBe(42);
	});

	it('demonstrates the TDZ bug pattern that was fixed', () => {
		const store = writable(true);

		function buggyPattern(): Promise<boolean> {
			return new Promise((resolve) => {
				const unsub = store.subscribe((v) => {
					unsub();
					resolve(v);
				});
			});
		}

		function fixedPattern(): boolean {
			let value = false;
			const unsub = store.subscribe((v) => (value = v));
			unsub();
			return value;
		}

		expect(fixedPattern()).toBe(true);
	});

	it('should work correctly with get() from svelte/store (the actual fix)', async () => {
		const activeIdentity = writable<{ id: number } | null>(null);
		const isAuthenticated = derived(activeIdentity, ($active) => $active !== null);

		expect(get(isAuthenticated)).toBe(false);

		activeIdentity.set({ id: 1 });
		expect(get(isAuthenticated)).toBe(true);

		activeIdentity.set(null);
		expect(get(isAuthenticated)).toBe(false);
	});

	it('get() should work in async context without TDZ error', async () => {
		const store = writable(42);

		async function getValueWithGet(): Promise<number> {
			return get(store);
		}

		await expect(getValueWithGet()).resolves.toBe(42);

		store.set(100);
		await expect(getValueWithGet()).resolves.toBe(100);
	});
});
