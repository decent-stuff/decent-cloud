import type { AccountExternalKey } from '$lib/types/generated/AccountExternalKey';

export const ONBOARDING_SESSION_KEY = 'first_login_onboarding_completed';

export type OnboardingStep = 1 | 2 | 3;
export type ActivationAction = 'marketplace' | 'provider';

export function isOnboardingCompleted(storage: Pick<Storage, 'getItem'>): boolean {
	return storage.getItem(ONBOARDING_SESSION_KEY) === 'true';
}

export function completeOnboarding(storage: Pick<Storage, 'setItem'>): void {
	storage.setItem(ONBOARDING_SESSION_KEY, 'true');
}

export function nextStep(step: OnboardingStep): OnboardingStep {
	if (step === 1) return 2;
	if (step === 2) return 3;
	return 3;
}

export function hasSshExternalKey(keys: AccountExternalKey[]): boolean {
	return keys.some((key) => key.keyType.startsWith('ssh-'));
}

export function getActivationActionHref(action: ActivationAction): string {
	if (action === 'provider') return '/dashboard/provider/support';
	return '/dashboard/marketplace';
}
