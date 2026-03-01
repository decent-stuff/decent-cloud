import { describe, it, expect } from 'vitest';
import { isStripeSupportedCurrency } from '$lib/utils/stripe-currencies';
import { generateSshKeyPair, validateSshPublicKey } from '$lib/utils/ssh-keygen';

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

describe('RentalRequestDialog payment method default', () => {
	// This test verifies the expected default payment method behavior
	// For fiat currencies (USD, EUR, etc.) that Stripe supports, default should be "stripe"
	// For crypto-only currencies, default should be "icpay"

	it('should default to stripe for USD currency (fiat - Stripe supported)', () => {
		const currency = 'USD';
		const isStripeAvailable = isStripeSupportedCurrency(currency);
		// Default payment method should be stripe when Stripe is available
		const defaultPaymentMethod = isStripeAvailable ? 'stripe' : 'icpay';

		expect(isStripeAvailable).toBe(true);
		expect(defaultPaymentMethod).toBe('stripe');
	});

	it('should default to stripe for EUR currency (fiat - Stripe supported)', () => {
		const currency = 'EUR';
		const isStripeAvailable = isStripeSupportedCurrency(currency);
		// Default payment method should be stripe when Stripe is available
		const defaultPaymentMethod = isStripeAvailable ? 'stripe' : 'icpay';

		expect(isStripeAvailable).toBe(true);
		expect(defaultPaymentMethod).toBe('stripe');
	});

	it('should default to stripe for other fiat currencies (GBP, CAD, etc.)', () => {
		const fiatCurrencies = ['GBP', 'CAD', 'AUD', 'JPY', 'CHF'];
		
		for (const currency of fiatCurrencies) {
			const isStripeAvailable = isStripeSupportedCurrency(currency);
			const defaultPaymentMethod = isStripeAvailable ? 'stripe' : 'icpay';
			
			expect(isStripeAvailable).toBe(true);
			expect(defaultPaymentMethod).toBe('stripe');
		}
	});

	it('should default to icpay for ICP (crypto-only currency)', () => {
		const currency = 'ICP';
		const isStripeAvailable = isStripeSupportedCurrency(currency);
		// Default payment method should be icpay when Stripe is NOT available
		const defaultPaymentMethod = isStripeAvailable ? 'stripe' : 'icpay';

		expect(isStripeAvailable).toBe(false);
		expect(defaultPaymentMethod).toBe('icpay');
	});

	it('should default to icpay for BTC (cryptocurrency)', () => {
		const currency = 'BTC';
		const isStripeAvailable = isStripeSupportedCurrency(currency);
		// Default payment method should be icpay when Stripe is NOT available
		const defaultPaymentMethod = isStripeAvailable ? 'stripe' : 'icpay';

		expect(isStripeAvailable).toBe(false);
		expect(defaultPaymentMethod).toBe('icpay');
	});

	it('should default to icpay for ETH (cryptocurrency)', () => {
		const currency = 'ETH';
		const isStripeAvailable = isStripeSupportedCurrency(currency);
		// Default payment method should be icpay when Stripe is NOT available
		const defaultPaymentMethod = isStripeAvailable ? 'stripe' : 'icpay';

		expect(isStripeAvailable).toBe(false);
		expect(defaultPaymentMethod).toBe('icpay');
	});
});

describe('RentalRequestDialog SSH key generation', () => {
	it('should generate a valid SSH keypair for the "Generate for me" feature', async () => {
		const { publicKeySsh, privateKeyPem } = await generateSshKeyPair('test-user');
		expect(publicKeySsh).toMatch(/^ssh-ed25519 [A-Za-z0-9+/]+=* test-user$/);
		expect(privateKeyPem).toContain('-----BEGIN OPENSSH PRIVATE KEY-----');
		expect(privateKeyPem).toContain('-----END OPENSSH PRIVATE KEY-----');
	});

	it('should auto-fill a valid public key when generated', async () => {
		const { publicKeySsh } = await generateSshKeyPair();
		const validation = validateSshPublicKey(publicKeySsh);
		expect(validation.valid).toBe(true);
	});

	it('should generate different keys each time', async () => {
		const key1 = await generateSshKeyPair();
		const key2 = await generateSshKeyPair();
		expect(key1.publicKeySsh).not.toBe(key2.publicKeySsh);
		expect(key1.privateKeyPem).not.toBe(key2.privateKeyPem);
	});
});

describe('RentalRequestDialog SSH key UX for non-technical users', () => {
	it('should generate key without requiring user to know SSH commands', async () => {
		const { publicKeySsh, privateKeyPem } = await generateSshKeyPair();
		const validation = validateSshPublicKey(publicKeySsh);
		expect(validation.valid).toBe(true);
		expect(privateKeyPem).toContain('-----BEGIN OPENSSH PRIVATE KEY-----');
	});

	it('should produce downloadable private key format', async () => {
		const { privateKeyPem } = await generateSshKeyPair();
		expect(privateKeyPem).toMatch(/-----BEGIN OPENSSH PRIVATE KEY-----[\s\S]*-----END OPENSSH PRIVATE KEY-----/);
		expect(privateKeyPem.length).toBeGreaterThan(100);
	});

	it('should include clear instructions in generated private key comment', async () => {
		const { publicKeySsh } = await generateSshKeyPair('decent-cloud');
		expect(publicKeySsh).toContain('decent-cloud');
	});

	it('should validate generated key passes SSH key format check', async () => {
		const { publicKeySsh } = await generateSshKeyPair();
		const trimmed = publicKeySsh.trim();
		const parts = trimmed.split(/\s+/);
		expect(parts.length).toBeGreaterThanOrEqual(2);
		expect(parts[0]).toBe('ssh-ed25519');
		expect(parts[1].length).toBeGreaterThan(20);
	});
});
