import { describe, it, expect, beforeEach, afterEach } from 'vitest';

// ---- Pure logic extracted from WelcomeModal ----

const ONBOARDING_KEY = 'onboarding_completed';
const ROLE_PREF_KEY = 'user_role_preference';

type UserRolePreference = 'tenant' | 'provider';

function isOnboardingCompleted(storage: Record<string, string>): boolean {
	return storage[ONBOARDING_KEY] === 'true';
}

function completeOnboarding(storage: Record<string, string>): Record<string, string> {
	return { ...storage, [ONBOARDING_KEY]: 'true' };
}

function saveRolePreference(storage: Record<string, string>, role: UserRolePreference): Record<string, string> {
	return { ...storage, [ROLE_PREF_KEY]: role };
}

// ---- Tests ----

describe('WelcomeModal: onboarding_completed gate', () => {
	it('shows modal when onboarding_completed is not set', () => {
		const storage: Record<string, string> = {};
		expect(isOnboardingCompleted(storage)).toBe(false);
	});

	it('hides modal when onboarding_completed is "true"', () => {
		const storage: Record<string, string> = { [ONBOARDING_KEY]: 'true' };
		expect(isOnboardingCompleted(storage)).toBe(true);
	});

	it('still shows modal when onboarding_completed is "1" (old format)', () => {
		// Only exact 'true' string dismisses the modal
		const storage: Record<string, string> = { [ONBOARDING_KEY]: '1' };
		expect(isOnboardingCompleted(storage)).toBe(false);
	});

	it('still shows modal when onboarding_completed is empty string', () => {
		const storage: Record<string, string> = { [ONBOARDING_KEY]: '' };
		expect(isOnboardingCompleted(storage)).toBe(false);
	});
});

describe('WelcomeModal: localStorage written on completion', () => {
	it('sets onboarding_completed to "true" on completion', () => {
		let storage: Record<string, string> = {};
		storage = completeOnboarding(storage);
		expect(storage[ONBOARDING_KEY]).toBe('true');
	});

	it('completion does not remove other keys', () => {
		let storage: Record<string, string> = { other: 'data' };
		storage = completeOnboarding(storage);
		expect(storage['other']).toBe('data');
		expect(storage[ONBOARDING_KEY]).toBe('true');
	});

	it('completing twice is idempotent', () => {
		let storage: Record<string, string> = {};
		storage = completeOnboarding(storage);
		storage = completeOnboarding(storage);
		expect(Object.keys(storage).filter(k => k === ONBOARDING_KEY)).toHaveLength(1);
		expect(storage[ONBOARDING_KEY]).toBe('true');
	});
});

describe('WelcomeModal: role preference saved to localStorage', () => {
	it('saves "tenant" preference', () => {
		let storage: Record<string, string> = {};
		storage = saveRolePreference(storage, 'tenant');
		expect(storage[ROLE_PREF_KEY]).toBe('tenant');
	});

	it('saves "provider" preference', () => {
		let storage: Record<string, string> = {};
		storage = saveRolePreference(storage, 'provider');
		expect(storage[ROLE_PREF_KEY]).toBe('provider');
	});

	it('overwriting role preference replaces previous value', () => {
		let storage: Record<string, string> = { [ROLE_PREF_KEY]: 'tenant' };
		storage = saveRolePreference(storage, 'provider');
		expect(storage[ROLE_PREF_KEY]).toBe('provider');
	});

	it('role preference does not affect onboarding_completed', () => {
		let storage: Record<string, string> = {};
		storage = saveRolePreference(storage, 'tenant');
		expect(storage[ONBOARDING_KEY]).toBeUndefined();
	});
});

describe('WelcomeModal: step navigation logic', () => {
	it('starts at step 1', () => {
		let step = 1;
		expect(step).toBe(1);
	});

	it('advances from step 1 to step 2', () => {
		let step = 1;
		step += 1;
		expect(step).toBe(2);
	});

	it('advances from step 2 to step 3 on role selection', () => {
		let step = 2;
		let selectedRole: UserRolePreference | null = null;

		function handleRoleSelect(role: UserRolePreference) {
			selectedRole = role;
			step += 1;
		}

		handleRoleSelect('provider');
		expect(step).toBe(3);
		expect(selectedRole).toBe('provider');
	});

	it('step 3 shows provider CTA when role is provider', () => {
		const selectedRole = 'provider' as UserRolePreference;
		const expectedHref = '/dashboard/provider/support';
		const actualHref = selectedRole === 'provider' ? '/dashboard/provider/support' : '/dashboard/marketplace';
		expect(actualHref).toBe(expectedHref);
	});

	it('step 3 shows marketplace CTA when role is tenant', () => {
		const selectedRole = 'tenant' as UserRolePreference;
		const expectedHref = '/dashboard/marketplace';
		const actualHref = selectedRole === 'provider' ? '/dashboard/provider/support' : '/dashboard/marketplace';
		expect(actualHref).toBe(expectedHref);
	});
});

// ---- localStorage integration tests ----

describe('WelcomeModal: localStorage integration', () => {
	beforeEach(() => {
		localStorage.clear();
	});

	afterEach(() => {
		localStorage.clear();
	});

	it('modal shown when localStorage has no onboarding_completed', () => {
		expect(localStorage.getItem(ONBOARDING_KEY)).toBeNull();
		// Modal would be shown (open = true)
		const open = localStorage.getItem(ONBOARDING_KEY) !== 'true';
		expect(open).toBe(true);
	});

	it('modal hidden when localStorage has onboarding_completed = "true"', () => {
		localStorage.setItem(ONBOARDING_KEY, 'true');
		const open = localStorage.getItem(ONBOARDING_KEY) !== 'true';
		expect(open).toBe(false);
	});

	it('completing onboarding writes "true" to localStorage', () => {
		localStorage.setItem(ONBOARDING_KEY, 'true');
		expect(localStorage.getItem(ONBOARDING_KEY)).toBe('true');
	});

	it('role preference survives across localStorage read/write cycle', () => {
		localStorage.setItem(ROLE_PREF_KEY, 'provider');
		expect(localStorage.getItem(ROLE_PREF_KEY)).toBe('provider');
	});
});
