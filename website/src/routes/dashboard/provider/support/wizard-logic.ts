// Pure wizard step logic - no DOM dependencies, fully testable

export const WIZARD_STEP_COUNT = 3;
export const WIZARD_STORAGE_KEY = 'provider-setup-wizard-step';

export interface WizardStepData {
	chatwootAccountExists: boolean;
	contactsCount: number;
	notifyEmail: boolean;
	notifyTelegram: boolean;
	notifySms: boolean;
	onboardingCompleted: boolean;
}

export const wizardStepLabels: readonly string[] = [
	'Support Portal',
	'Contacts & Notifications',
	'Help Center Profile',
];

/** Read the persisted wizard step from localStorage, clamped to [1, WIZARD_STEP_COUNT]. */
export function getWizardStep(storage: Pick<Storage, 'getItem'>): number {
	const raw = storage.getItem(WIZARD_STORAGE_KEY);
	if (raw === null) return 1;
	const parsed = parseInt(raw, 10);
	if (!Number.isInteger(parsed)) return 1;
	return Math.min(Math.max(parsed, 1), WIZARD_STEP_COUNT);
}

/** Persist the current step to localStorage. */
export function setWizardStep(storage: Pick<Storage, 'setItem'>, step: number): void {
	storage.setItem(WIZARD_STORAGE_KEY, String(step));
}

/** Returns true if the given 1-based step has been completed. */
export function isStepComplete(step: number, data: WizardStepData): boolean {
	switch (step) {
		case 1:
			return data.chatwootAccountExists;
		case 2:
			return (
				data.contactsCount > 0 &&
				(data.notifyEmail || data.notifyTelegram || data.notifySms)
			);
		case 3:
			return data.onboardingCompleted;
		default:
			return false;
	}
}

/** Returns true if the Back button should be shown for the given 1-based step. */
export function canGoBack(step: number): boolean {
	return step > 1;
}
