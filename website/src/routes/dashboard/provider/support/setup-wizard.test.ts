import { describe, it, expect } from 'vitest';
import {
	getWizardStep,
	setWizardStep,
	isStepComplete,
	canGoBack,
	wizardStepLabels,
	WIZARD_STEP_COUNT,
	WIZARD_STORAGE_KEY,
	type WizardStepData,
} from './wizard-logic';

// Minimal localStorage stub
function makeStorage(initial: Record<string, string> = {}): Pick<Storage, 'getItem' | 'setItem'> {
	const store: Record<string, string> = { ...initial };
	return {
		getItem: (key: string) => store[key] ?? null,
		setItem: (key: string, value: string) => {
			store[key] = value;
		},
	};
}

const completeStep1Data: WizardStepData = {
	chatwootAccountExists: true,
	contactsCount: 1,
	notifyEmail: true,
	notifyTelegram: false,
	notifySms: false,
	onboardingCompleted: true,
};

// ---- getWizardStep ----

describe('getWizardStep', () => {
	it('returns 1 when no value is stored', () => {
		const storage = makeStorage();
		expect(getWizardStep(storage)).toBe(1);
	});

	it('returns stored step 1', () => {
		const storage = makeStorage({ [WIZARD_STORAGE_KEY]: '1' });
		expect(getWizardStep(storage)).toBe(1);
	});

	it('returns stored step 2', () => {
		const storage = makeStorage({ [WIZARD_STORAGE_KEY]: '2' });
		expect(getWizardStep(storage)).toBe(2);
	});

	it('returns stored step 3', () => {
		const storage = makeStorage({ [WIZARD_STORAGE_KEY]: '3' });
		expect(getWizardStep(storage)).toBe(3);
	});

	it('clamps values below 1 to 1', () => {
		const storage = makeStorage({ [WIZARD_STORAGE_KEY]: '0' });
		expect(getWizardStep(storage)).toBe(1);
	});

	it('clamps values above WIZARD_STEP_COUNT to WIZARD_STEP_COUNT', () => {
		const storage = makeStorage({ [WIZARD_STORAGE_KEY]: '99' });
		expect(getWizardStep(storage)).toBe(WIZARD_STEP_COUNT);
	});

	it('returns 1 for non-numeric stored value', () => {
		const storage = makeStorage({ [WIZARD_STORAGE_KEY]: 'bad' });
		expect(getWizardStep(storage)).toBe(1);
	});
});

// ---- setWizardStep ----

describe('setWizardStep', () => {
	it('persists the step to storage', () => {
		const storage = makeStorage();
		setWizardStep(storage, 2);
		expect(getWizardStep(storage)).toBe(2);
	});
});

// ---- isStepComplete: step 1 ----

describe('isStepComplete step 1 (Support Portal)', () => {
	it('returns true when chatwootAccountExists is true', () => {
		expect(isStepComplete(1, { ...completeStep1Data, chatwootAccountExists: true })).toBe(true);
	});

	it('returns false when chatwootAccountExists is false', () => {
		expect(isStepComplete(1, { ...completeStep1Data, chatwootAccountExists: false })).toBe(false);
	});
});

// ---- isStepComplete: step 2 ----

describe('isStepComplete step 2 (Contacts & Notifications)', () => {
	const base: WizardStepData = {
		chatwootAccountExists: true,
		contactsCount: 1,
		notifyEmail: true,
		notifyTelegram: false,
		notifySms: false,
		onboardingCompleted: false,
	};

	it('returns true when contacts > 0 and email notification enabled', () => {
		expect(isStepComplete(2, { ...base, contactsCount: 1, notifyEmail: true })).toBe(true);
	});

	it('returns true when contacts > 0 and telegram notification enabled', () => {
		expect(isStepComplete(2, { ...base, notifyEmail: false, notifyTelegram: true })).toBe(true);
	});

	it('returns true when contacts > 0 and sms notification enabled', () => {
		expect(isStepComplete(2, { ...base, notifyEmail: false, notifySms: true })).toBe(true);
	});

	it('returns false when contactsCount is 0 even with notifications', () => {
		expect(isStepComplete(2, { ...base, contactsCount: 0, notifyEmail: true })).toBe(false);
	});

	it('returns false when no notification channel is enabled', () => {
		expect(
			isStepComplete(2, { ...base, notifyEmail: false, notifyTelegram: false, notifySms: false }),
		).toBe(false);
	});
});

// ---- isStepComplete: step 3 ----

describe('isStepComplete step 3 (Help Center Profile)', () => {
	it('returns true when onboardingCompleted is true', () => {
		expect(isStepComplete(3, { ...completeStep1Data, onboardingCompleted: true })).toBe(true);
	});

	it('returns false when onboardingCompleted is false', () => {
		expect(isStepComplete(3, { ...completeStep1Data, onboardingCompleted: false })).toBe(false);
	});
});

// ---- isStepComplete: invalid step ----

describe('isStepComplete invalid step', () => {
	it('returns false for step 0', () => {
		expect(isStepComplete(0, completeStep1Data)).toBe(false);
	});

	it('returns false for step 4', () => {
		expect(isStepComplete(4, completeStep1Data)).toBe(false);
	});
});

// ---- wizardStepLabels ----

describe('wizardStepLabels', () => {
	it('has exactly WIZARD_STEP_COUNT labels', () => {
		expect(wizardStepLabels.length).toBe(WIZARD_STEP_COUNT);
	});

	it('label for step 1 is "Support Portal"', () => {
		expect(wizardStepLabels[0]).toBe('Support Portal');
	});

	it('label for step 2 is "Contacts & Notifications"', () => {
		expect(wizardStepLabels[1]).toBe('Contacts & Notifications');
	});

	it('label for step 3 is "Help Center Profile"', () => {
		expect(wizardStepLabels[2]).toBe('Help Center Profile');
	});
});

// ---- canGoBack ----

describe('canGoBack', () => {
	it('returns false for step 1', () => {
		expect(canGoBack(1)).toBe(false);
	});

	it('returns true for step 2', () => {
		expect(canGoBack(2)).toBe(true);
	});

	it('returns true for step 3', () => {
		expect(canGoBack(3)).toBe(true);
	});
});
