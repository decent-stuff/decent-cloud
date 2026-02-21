import { describe, it, expect } from 'vitest';
import { validateStep1, validateStep2, validateStep3, type Step2State } from './offering-wizard';

describe('validateStep1', () => {
	it('returns null when both name and id are present', () => {
		expect(validateStep1('My Offering', 'my-offering')).toBeNull();
	});

	it('fails when offer name is empty', () => {
		expect(validateStep1('', 'my-offering')).toBe('Offer name is required');
	});

	it('fails when offer name is only whitespace', () => {
		expect(validateStep1('   ', 'my-offering')).toBe('Offer name is required');
	});

	it('fails when offering id is empty', () => {
		expect(validateStep1('My Offering', '')).toBe('Offering ID is required');
	});

	it('fails when offering id is only whitespace', () => {
		expect(validateStep1('My Offering', '   ')).toBe('Offering ID is required');
	});
});

describe('validateStep2', () => {
	const noAccount: Step2State = {
		selectedAccountId: '',
		selectedServerType: null,
		selectedLocation: null,
		selectedImage: null
	};

	it('passes when no account is selected (manual offering)', () => {
		expect(validateStep2(noAccount)).toBeNull();
	});

	it('passes when account and all selections are present', () => {
		const state: Step2State = {
			selectedAccountId: 'acc-1',
			selectedServerType: { name: 'cx22' },
			selectedLocation: { name: 'fsn1' },
			selectedImage: { name: 'ubuntu-22.04' }
		};
		expect(validateStep2(state)).toBeNull();
	});

	it('fails when account selected but server type missing', () => {
		const state: Step2State = {
			selectedAccountId: 'acc-1',
			selectedServerType: null,
			selectedLocation: { name: 'fsn1' },
			selectedImage: { name: 'ubuntu-22.04' }
		};
		expect(validateStep2(state)).toBe('Please select a server type');
	});

	it('fails when account selected but location missing', () => {
		const state: Step2State = {
			selectedAccountId: 'acc-1',
			selectedServerType: { name: 'cx22' },
			selectedLocation: null,
			selectedImage: { name: 'ubuntu-22.04' }
		};
		expect(validateStep2(state)).toBe('Please select a location');
	});

	it('fails when account selected but image missing', () => {
		const state: Step2State = {
			selectedAccountId: 'acc-1',
			selectedServerType: { name: 'cx22' },
			selectedLocation: { name: 'fsn1' },
			selectedImage: null
		};
		expect(validateStep2(state)).toBe('Please select an image');
	});
});

describe('validateStep3', () => {
	it('passes when price is positive', () => {
		expect(validateStep3(5.99)).toBeNull();
	});

	it('passes for minimum positive value', () => {
		expect(validateStep3(0.01)).toBeNull();
	});

	it('fails when price is null', () => {
		expect(validateStep3(null)).toBe('Monthly price must be greater than 0');
	});

	it('fails when price is zero', () => {
		expect(validateStep3(0)).toBe('Monthly price must be greater than 0');
	});

	it('fails when price is negative', () => {
		expect(validateStep3(-1)).toBe('Monthly price must be greater than 0');
	});
});
