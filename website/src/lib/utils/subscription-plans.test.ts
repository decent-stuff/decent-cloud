import { describe, it, expect } from 'vitest';
import { shouldShowTrialCopy } from '$lib/utils/subscription-plans';

/**
 * #441: subscription page copy must be internally consistent. A plan may only
 * advertise a "free trial" when the user can actually start it themselves
 * (stripe_price_id present → self-serve checkout). Plans whose only CTA is
 * "Contact Sales" must not promise a trial.
 */
describe('shouldShowTrialCopy', () => {
	it('shows trial copy for a self-serve plan with a trial window', () => {
		expect(shouldShowTrialCopy({ trialDays: 14, stripePriceId: 'price_abc' })).toBe(true);
	});

	it('hides trial copy for a contact-sales plan even when trialDays is set', () => {
		// This is the #441 bug: pro/enterprise carry trial_days=14 in the seed
		// but have no stripe_price_id, so their only CTA is "Contact Sales".
		expect(shouldShowTrialCopy({ trialDays: 14, stripePriceId: null })).toBe(false);
		expect(shouldShowTrialCopy({ trialDays: 14, stripePriceId: undefined })).toBe(false);
		expect(shouldShowTrialCopy({ trialDays: 14 })).toBe(false);
	});

	it('hides trial copy for plans with no trial window regardless of price id', () => {
		expect(shouldShowTrialCopy({ trialDays: 0, stripePriceId: 'price_abc' })).toBe(false);
		expect(shouldShowTrialCopy({ trialDays: 0, stripePriceId: null })).toBe(false);
	});
});
