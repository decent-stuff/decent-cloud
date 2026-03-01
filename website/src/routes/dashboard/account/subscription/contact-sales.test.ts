import { describe, it, expect } from 'vitest';

interface Plan {
	id: string;
	name: string;
	stripe_price_id: string | null;
}

function getContactSalesLink(): { href: string; subject: string } {
	return {
		href: 'mailto:support@decent-cloud.org',
		subject: 'Enterprise Plan Inquiry'
	};
}

function isContactSalesPlan(plan: Plan): boolean {
	return !plan.stripe_price_id && plan.id !== 'free';
}

describe('Contact Sales Link', () => {
	describe('contact link structure', () => {
		it('provides correct mailto link', () => {
			const link = getContactSalesLink();
			expect(link.href).toBe('mailto:support@decent-cloud.org');
		});

		it('includes enterprise inquiry subject', () => {
			const link = getContactSalesLink();
			expect(link.subject).toBe('Enterprise Plan Inquiry');
		});

		it('mailto link is properly formatted', () => {
			const link = getContactSalesLink();
			expect(link.href.startsWith('mailto:')).toBe(true);
			expect(link.href).toContain('@');
			expect(link.href).toContain('.');
		});
	});

	describe('plan eligibility for contact sales', () => {
		it('enterprise plan without stripe price shows contact sales', () => {
			const enterprisePlan: Plan = {
				id: 'enterprise',
				name: 'Enterprise',
				stripe_price_id: null
			};
			expect(isContactSalesPlan(enterprisePlan)).toBe(true);
		});

		it('pro plan without stripe price shows contact sales', () => {
			const proPlan: Plan = {
				id: 'pro',
				name: 'Pro',
				stripe_price_id: null
			};
			expect(isContactSalesPlan(proPlan)).toBe(true);
		});

		it('plan with stripe price does not show contact sales', () => {
			const basicPlan: Plan = {
				id: 'basic',
				name: 'Basic',
				stripe_price_id: 'price_123'
			};
			expect(isContactSalesPlan(basicPlan)).toBe(false);
		});

		it('free plan never shows contact sales', () => {
			const freePlan: Plan = {
				id: 'free',
				name: 'Free',
				stripe_price_id: null
			};
			expect(isContactSalesPlan(freePlan)).toBe(false);
		});
	});
});
