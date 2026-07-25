// Pure wizard step logic - no DOM dependencies, fully testable

export const WIZARD_STEP_COUNT = 3;
export const WIZARD_STORAGE_KEY = 'provider-setup-wizard-step';
export const WIZARD_STEP_QUERY_PARAM = 'step';

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
	return clampStep(parsed);
}

/** Persist the current step to localStorage. */
export function setWizardStep(storage: Pick<Storage, 'setItem'>, step: number): void {
	storage.setItem(WIZARD_STORAGE_KEY, String(step));
}

/**
 * Clamp a 1-based step to the valid range [1, WIZARD_STEP_COUNT]. Out-of-range
 * values (e.g. `?step=99`, `?step=0`, negatives) fold to the nearest bound so a
 * malformed deep-link can never put the wizard into a non-rendering state.
 */
export function clampStep(step: number): number {
	if (!Number.isInteger(step)) return 1;
	return Math.min(Math.max(step, 1), WIZARD_STEP_COUNT);
}

/**
 * Parse the `?step=N` query param from a URL search string. Returns the clamped
 * integer step when present and well-formed, or `null` when absent/non-integer
 * (so the caller can fall back to the persisted step). Accepts the search string
 * with OR without a leading `?`.
 *
 * Deep-link rule (kept simple and predictable — see C3 / audit #16): the query
 * param always wins over the persisted value when present and valid. A first-
 * time provider with no persisted state may therefore deep-link straight to
 * `?step=3`; a returning provider's persisted step still applies on a plain
 * reload (no `?step`). Negative/zero/non-integer values are ignored, not
 * clamped, so a typo like `?step=abc` falls back to the persisted step rather
 * than silently landing on step 1.
 */
export function parseStepParam(search: string): number | null {
	if (!search) return null;
	const params = new URLSearchParams(search.startsWith('?') ? search.slice(1) : search);
	const raw = params.get(WIZARD_STEP_QUERY_PARAM);
	if (raw === null || !/^-?\d+$/.test(raw)) return null;
	return clampStep(parseInt(raw, 10));
}

/**
 * Resolve the wizard's initial step on mount. A valid `?step=N` query param
 * wins (deep-link intent); otherwise the persisted localStorage step is used
 * (returning-provider behavior). The caller should persist the resolved value
 * so a reload without `?step` stays put.
 */
export function resolveInitialStep(
	search: string,
	storage: Pick<Storage, 'getItem'>,
): number {
	const fromUrl = parseStepParam(search);
	return fromUrl === null ? getWizardStep(storage) : fromUrl;
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
