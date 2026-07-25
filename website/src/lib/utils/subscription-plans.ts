/**
 * Subscription-plan display helpers.
 *
 * The plan catalog mixes self-serve (Stripe price id attached) and
 * sales-assisted (no price id → "Contact Sales") tiers. Trial copy must only
 * be advertised on a plan the user can actually start themselves, otherwise the
 * page promises a self-serve trial whose only CTA is "Contact Sales". See #441.
 */

export interface TrialEligiblePlan {
	/** Trial window length from the plan row (subscription_plans.trial_days). */
	trialDays: number;
	/** Stripe price id; present only when self-serve checkout is wired. */
	stripePriceId?: string | null;
}

/**
 * Whether the "{N}-day free trial" line should render for a plan. True only
 * when the plan both offers a trial AND has a self-serve checkout path
 * (stripe_price_id). Contact-sales-only plans never advertise a trial.
 */
export function shouldShowTrialCopy(plan: TrialEligiblePlan): boolean {
	return plan.trialDays > 0 && Boolean(plan.stripePriceId);
}
