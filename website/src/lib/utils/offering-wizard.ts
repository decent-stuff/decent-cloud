/**
 * Validation functions for the offering creation wizard.
 * Each returns null on success or an error string on failure.
 */

export interface Step2State {
	selectedAccountId: string;
	selectedServerType: { name: string } | null;
	selectedLocation: { name: string } | null;
	selectedImage: { name: string } | null;
}

/** Step 1: Basics — name and ID are required */
export function validateStep1(offerName: string, offeringId: string): string | null {
	if (!offerName.trim()) return 'Offer name is required';
	if (!offeringId.trim()) return 'Offering ID is required';
	return null;
}

/**
 * Step 2: Infrastructure — if an account is selected, server type + location + image must all be set.
 * If no account is selected, proceed (manual / no-provisioner offering).
 */
export function validateStep2(state: Step2State): string | null {
	if (!state.selectedAccountId) return null;
	if (!state.selectedServerType) return 'Please select a server type';
	if (!state.selectedLocation) return 'Please select a location';
	if (!state.selectedImage) return 'Please select an image';
	return null;
}

/** Step 3: Pricing — monthly price must be > 0 */
export function validateStep3(monthlyPrice: number | null): string | null {
	if (monthlyPrice === null || monthlyPrice <= 0) return 'Monthly price must be greater than 0';
	return null;
}
