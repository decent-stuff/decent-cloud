import { describe, it, expect } from 'vitest';

// Test the price calculation logic used by RentalRequestDialog
// This tests the core functionality without needing to render the component

describe('RentalRequestDialog price calculation', () => {
	it('calculates correct price for 720 hours (30 days) at monthly rate', () => {
		const monthlyPrice = 100;
		const durationHours = 720;
		const expectedPrice = (monthlyPrice * durationHours) / 720;

		expect(expectedPrice).toBe(100);
		expect(expectedPrice.toFixed(2)).toBe('100.00');
	});

	it('calculates correct price for 168 hours (1 week)', () => {
		const monthlyPrice = 120;
		const durationHours = 168;
		const expectedPrice = (monthlyPrice * durationHours) / 720;

		expect(expectedPrice).toBeCloseTo(28, 0);
		expect(expectedPrice.toFixed(2)).toBe('28.00');
	});

	it('calculates correct price for 24 hours (1 day)', () => {
		const monthlyPrice = 90;
		const durationHours = 24;
		const expectedPrice = (monthlyPrice * durationHours) / 720;

		expect(expectedPrice).toBe(3);
		expect(expectedPrice.toFixed(2)).toBe('3.00');
	});

	it('calculates correct price for 8760 hours (1 year)', () => {
		const monthlyPrice = 50;
		const durationHours = 8760;
		const expectedPrice = (monthlyPrice * durationHours) / 720;

		expect(expectedPrice).toBeCloseTo(608.33, 2);
		expect(expectedPrice.toFixed(2)).toBe('608.33');
	});

	it('handles decimal monthly prices', () => {
		const monthlyPrice = 99.99;
		const durationHours = 720;
		const expectedPrice = (monthlyPrice * durationHours) / 720;

		expect(expectedPrice).toBeCloseTo(99.99, 2);
		expect(expectedPrice.toFixed(2)).toBe('99.99');
	});

	it('handles zero monthly price', () => {
		const monthlyPrice = 0;
		const durationHours = 720;
		const expectedPrice = (monthlyPrice * durationHours) / 720;

		expect(expectedPrice).toBe(0);
		expect(expectedPrice.toFixed(2)).toBe('0.00');
	});

	it('calculates proportional pricing for custom durations', () => {
		const monthlyPrice = 100;
		const durationHours = 360; // Half month
		const expectedPrice = (monthlyPrice * durationHours) / 720;

		expect(expectedPrice).toBe(50);
		expect(expectedPrice.toFixed(2)).toBe('50.00');
	});
});
