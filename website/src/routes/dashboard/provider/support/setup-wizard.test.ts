import { describe, it, expect } from 'vitest';
import {
	getWizardStep,
	setWizardStep,
	isStepComplete,
	canGoBack,
	wizardStepLabels,
	WIZARD_STEP_COUNT,
	WIZARD_STORAGE_KEY,
	clampStep,
	parseStepParam,
	resolveInitialStep,
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

// ---- clampStep ----

describe('clampStep', () => {
	it('passes through in-range integers unchanged', () => {
		expect(clampStep(1)).toBe(1);
		expect(clampStep(2)).toBe(2);
		expect(clampStep(3)).toBe(3);
	});

	it('clamps values below 1 up to 1', () => {
		expect(clampStep(0)).toBe(1);
		expect(clampStep(-5)).toBe(1);
	});

	it('clamps values above WIZARD_STEP_COUNT down to WIZARD_STEP_COUNT', () => {
		expect(clampStep(WIZARD_STEP_COUNT + 1)).toBe(WIZARD_STEP_COUNT);
		expect(clampStep(99)).toBe(WIZARD_STEP_COUNT);
	});

	it('returns 1 for non-integer input', () => {
		expect(clampStep(NaN)).toBe(1);
		expect(clampStep(2.5)).toBe(1);
	});
});

// ---- parseStepParam (?step=N deep-link, audit #16) ----

describe('parseStepParam', () => {
	it('returns the clamped step for a valid ?step=N', () => {
		expect(parseStepParam('?step=1')).toBe(1);
		expect(parseStepParam('?step=2')).toBe(2);
		expect(parseStepParam('?step=3')).toBe(3);
	});

	it('accepts a search string without the leading ?', () => {
		expect(parseStepParam('step=3')).toBe(3);
	});

	it('preserves other params alongside step', () => {
		expect(parseStepParam('?foo=bar&step=2&baz=1')).toBe(2);
	});

	it('clamps out-of-range values to the nearest bound', () => {
		expect(parseStepParam('?step=0')).toBe(1);
		expect(parseStepParam('?step=99')).toBe(WIZARD_STEP_COUNT);
		expect(parseStepParam('?step=-3')).toBe(1);
	});

	it('returns null when the param is absent', () => {
		expect(parseStepParam('')).toBeNull();
		expect(parseStepParam('?foo=bar')).toBeNull();
	});

	it('returns null for non-integer values (typo falls back to persisted step)', () => {
		expect(parseStepParam('?step=abc')).toBeNull();
		expect(parseStepParam('?step=')).toBeNull();
		expect(parseStepParam('?step=2.5')).toBeNull();
	});
});

// ---- resolveInitialStep (?step=N deep-link precedence) ----

describe('resolveInitialStep', () => {
	it('uses the ?step=N value when present and valid', () => {
		const storage = makeStorage({ [WIZARD_STORAGE_KEY]: '1' });
		expect(resolveInitialStep('?step=3', storage)).toBe(3);
	});

	it('falls back to the persisted localStorage step when ?step is absent', () => {
		const storage = makeStorage({ [WIZARD_STORAGE_KEY]: '2' });
		expect(resolveInitialStep('', storage)).toBe(2);
		expect(resolveInitialStep('?other=1', storage)).toBe(2);
	});

	it('falls back to the persisted step when ?step is non-integer (typo)', () => {
		const storage = makeStorage({ [WIZARD_STORAGE_KEY]: '2' });
		expect(resolveInitialStep('?step=abc', storage)).toBe(2);
	});

	it('defaults to step 1 for a first-time provider with no ?step and no storage', () => {
		const storage = makeStorage();
		expect(resolveInitialStep('', storage)).toBe(1);
	});

	it('clamps an out-of-range ?step to the nearest bound', () => {
		const storage = makeStorage({ [WIZARD_STORAGE_KEY]: '2' });
		expect(resolveInitialStep('?step=99', storage)).toBe(WIZARD_STEP_COUNT);
		expect(resolveInitialStep('?step=0', storage)).toBe(1);
	});

	it('deep-link wins over a different persisted step (explicit URL intent)', () => {
		const storage = makeStorage({ [WIZARD_STORAGE_KEY]: '3' });
		expect(resolveInitialStep('?step=1', storage)).toBe(1);
	});
});
