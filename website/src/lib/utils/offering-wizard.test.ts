import { describe, it, expect } from 'vitest';
import {
	validateStep1,
	validateStep2,
	validateStep3,
	suggestMonthlyPrice,
	DEFAULT_MARKUP,
	type Step2State,
} from './offering-wizard';

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

describe('DEFAULT_MARKUP', () => {
	it('is 1.15 (the product decision on GH #442: 15% over Hetzner cost)', () => {
		expect(DEFAULT_MARKUP).toBe(1.15);
	});
});

describe('suggestMonthlyPrice', () => {
	it('returns cost × 1.15 rounded to 2 decimals for a positive cost', () => {
		// cx22-style monthly cost: 4.59 × 1.15 = 5.2785 → 5.28
		expect(suggestMonthlyPrice(4.59)).toBe(5.28);
	});

	it('rounds to 2 decimals via Math.round', () => {
		// 10.00 × 1.15 = 11.5 exactly → stays 11.5
		expect(suggestMonthlyPrice(10)).toBe(11.5);
		// 7.45 × 1.15 = 8.5675 → 8.57
		expect(suggestMonthlyPrice(7.45)).toBe(8.57);
	});

	it('returns null when cost is undefined', () => {
		expect(suggestMonthlyPrice(undefined)).toBeNull();
	});

	it('returns null when cost is null', () => {
		expect(suggestMonthlyPrice(null)).toBeNull();
	});

	it('returns null when cost is zero', () => {
		expect(suggestMonthlyPrice(0)).toBeNull();
	});

	it('returns null when cost is negative', () => {
		expect(suggestMonthlyPrice(-1)).toBeNull();
	});
});
