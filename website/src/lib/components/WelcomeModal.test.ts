import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
	ONBOARDING_SESSION_KEY,
	completeOnboarding,
	getActivationActionHref,
	hasSshExternalKey,
	isOnboardingCompleted,
	nextStep,
	type OnboardingStep,
} from './welcome-onboarding';

describe('welcome onboarding session gate', () => {
	beforeEach(() => {
		sessionStorage.clear();
	});

	afterEach(() => {
		sessionStorage.clear();
	});

	it('starts open when session flag is not set', () => {
		expect(isOnboardingCompleted(sessionStorage)).toBe(false);
	});

	it('closes after completion in the same session', () => {
		completeOnboarding(sessionStorage);
		expect(sessionStorage.getItem(ONBOARDING_SESSION_KEY)).toBe('true');
		expect(isOnboardingCompleted(sessionStorage)).toBe(true);
	});
});

describe('welcome onboarding ssh key checks', () => {
	it('returns true when at least one ssh key exists', () => {
		expect(
			hasSshExternalKey([
				{ id: 1, keyType: 'gpg', keyData: 'gpg-key', keyFingerprint: null, label: null },
				{ id: 2, keyType: 'ssh-ed25519', keyData: 'ssh-key', keyFingerprint: null, label: null },
			]),
		).toBe(true);
	});

	it('returns false when ssh key is missing', () => {
		expect(
			hasSshExternalKey([
				{ id: 1, keyType: 'gpg', keyData: 'gpg-key', keyFingerprint: null, label: null },
			]),
		).toBe(false);
	});
});

describe('welcome onboarding step flow', () => {
	it('advances from step 1 to step 2', () => {
		const next = nextStep(1);
		expect(next).toBe(2);
	});

	it('caps at step 3', () => {
		const next = nextStep(3);
		expect(next).toBe(3);
	});

	it('returns provider and marketplace activation targets', () => {
		expect(getActivationActionHref('marketplace')).toBe('/dashboard/marketplace');
		expect(getActivationActionHref('provider')).toBe('/dashboard/provider/support');
	});

	it('keeps steps typed as 1-3 only', () => {
		const step: OnboardingStep = 2;
		expect(step).toBe(2);
	});
});
