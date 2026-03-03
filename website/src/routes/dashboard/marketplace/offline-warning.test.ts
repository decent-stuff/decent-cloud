import { describe, it, expect } from 'vitest';

type OfferingForRentButton = {
	is_example?: boolean;
	provider_online?: boolean | null;
	offering_source?: string;
	external_checkout_url?: string;
};

function getRentButtonState(offering: OfferingForRentButton): {
	disabled: boolean;
	label: string;
	title: string;
} {
	if (offering.offering_source === 'seeded' && offering.external_checkout_url) {
		return { disabled: false, label: 'Visit Provider', title: '' };
	}
	if (offering.is_example) {
		return { disabled: true, label: 'Demo only', title: 'Demo only — not available for rent' };
	}
	if (offering.provider_online === false) {
		return {
			disabled: true,
			label: 'Offline',
			title: 'Provider is offline. Your request will be queued until they return.'
		};
	}
	return { disabled: false, label: 'Rent', title: '' };
}

describe('rent button: offline state', () => {
	it('shows disabled "Offline" button when provider_online is false', () => {
		const offering: OfferingForRentButton = { provider_online: false };
		const state = getRentButtonState(offering);
		expect(state.disabled).toBe(true);
		expect(state.label).toBe('Offline');
		expect(state.title).toContain('queued');
	});

	it('shows enabled "Rent" button when provider_online is null (status unknown)', () => {
		const offering: OfferingForRentButton = { provider_online: null };
		const state = getRentButtonState(offering);
		expect(state.disabled).toBe(false);
		expect(state.label).toBe('Rent');
	});

	it('shows enabled "Rent" button when provider_online is undefined (status unknown)', () => {
		const offering: OfferingForRentButton = { provider_online: undefined };
		const state = getRentButtonState(offering);
		expect(state.disabled).toBe(false);
		expect(state.label).toBe('Rent');
	});

	it('shows enabled "Rent" button when provider_online is true', () => {
		const offering: OfferingForRentButton = { provider_online: true };
		const state = getRentButtonState(offering);
		expect(state.disabled).toBe(false);
		expect(state.label).toBe('Rent');
	});
});

describe('rent button: demo state', () => {
	it('shows disabled "Demo only" button for example offerings (takes precedence)', () => {
		const offering: OfferingForRentButton = { is_example: true, provider_online: true };
		const state = getRentButtonState(offering);
		expect(state.disabled).toBe(true);
		expect(state.label).toBe('Demo only');
	});

	it('shows disabled "Demo only" even when offline', () => {
		const offering: OfferingForRentButton = { is_example: true, provider_online: false };
		const state = getRentButtonState(offering);
		expect(state.disabled).toBe(true);
		expect(state.label).toBe('Demo only');
	});
});

describe('rent button: external checkout', () => {
	it('shows "Visit Provider" link for seeded offerings with external URL', () => {
		const offering: OfferingForRentButton = {
			offering_source: 'seeded',
			external_checkout_url: 'https://example.com/checkout'
		};
		const state = getRentButtonState(offering);
		expect(state.label).toBe('Visit Provider');
		expect(state.disabled).toBe(false);
	});

	it('shows normal rent button for seeded offerings without external URL', () => {
		const offering: OfferingForRentButton = {
			offering_source: 'seeded',
			provider_online: true
		};
		const state = getRentButtonState(offering);
		expect(state.label).toBe('Rent');
		expect(state.disabled).toBe(false);
	});
});

describe('rent button: normal state', () => {
	it('shows enabled "Rent" button for online non-demo offering', () => {
		const offering: OfferingForRentButton = { provider_online: true };
		const state = getRentButtonState(offering);
		expect(state.disabled).toBe(false);
		expect(state.label).toBe('Rent');
		expect(state.title).toBe('');
	});
});
