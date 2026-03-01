import { describe, it, expect } from 'vitest';
import {
	LIFECYCLE_STAGES,
	STAGE_TIMING,
	getStageIndex,
	getStageTiming,
	getStageTimingWithProvider,
	getStageExplanation,
	getNextStepInfo,
	stageElapsedMinutes,
	formatElapsed,
	isStageOverdue,
	getProgressSummary,
	type ProviderTimingEstimate,
} from './contract-progress';

describe('LIFECYCLE_STAGES', () => {
	it('has 4 stages in correct order', () => {
		expect(LIFECYCLE_STAGES).toHaveLength(4);
		expect(LIFECYCLE_STAGES[0].key).toBe('payment');
		expect(LIFECYCLE_STAGES[1].key).toBe('provider');
		expect(LIFECYCLE_STAGES[2].key).toBe('provisioning');
		expect(LIFECYCLE_STAGES[3].key).toBe('ready');
	});
});

describe('getStageIndex', () => {
	it('returns -1 for terminal states', () => {
		expect(getStageIndex('cancelled')).toBe(-1);
		expect(getStageIndex('rejected')).toBe(-1);
		expect(getStageIndex('CANCELLED')).toBe(-1);
		expect(getStageIndex('REJECTED')).toBe(-1);
	});

	it('returns 0 for payment-pending states', () => {
		expect(getStageIndex('requested', 'pending')).toBe(0);
		expect(getStageIndex('requested', 'failed')).toBe(0);
		expect(getStageIndex('REQUESTED', 'PENDING')).toBe(0);
	});

	it('returns 1 for provider review states', () => {
		expect(getStageIndex('requested', 'succeeded')).toBe(1);
		expect(getStageIndex('requested')).toBe(1);
		expect(getStageIndex('pending')).toBe(1);
	});

	it('returns 2 for provisioning states', () => {
		expect(getStageIndex('accepted')).toBe(2);
		expect(getStageIndex('provisioning')).toBe(2);
	});

	it('returns 3 for ready states', () => {
		expect(getStageIndex('provisioned')).toBe(3);
		expect(getStageIndex('active')).toBe(3);
	});

	it('handles case-insensitivity', () => {
		expect(getStageIndex('ACTIVE')).toBe(3);
		expect(getStageIndex('ProVisioned')).toBe(3);
		expect(getStageIndex('PROVISIONING')).toBe(2);
	});
});

describe('getStageTiming', () => {
	it('returns timing for requested + pending payment', () => {
		const timing = getStageTiming('requested', 'pending');
		expect(timing).not.toBeNull();
		expect(timing?.max).toBe(5);
		expect(timing?.label).toContain('Payment');
	});

	it('returns timing for provisioning', () => {
		const timing = getStageTiming('provisioning');
		expect(timing).not.toBeNull();
		expect(timing?.min).toBe(5);
		expect(timing?.max).toBe(20);
		expect(timing?.label).toContain('VM setup');
	});

	it('returns null for terminal states', () => {
		expect(getStageTiming('cancelled')).toBeNull();
		expect(getStageTiming('rejected')).toBeNull();
	});

	it('returns timing with max 0 for active state', () => {
		const timing = getStageTiming('active');
		expect(timing?.max).toBe(0);
	});
});

describe('getNextStepInfo', () => {
	it('returns payment prompt for pending payment', () => {
		const info = getNextStepInfo('requested', 'pending');
		expect(info).toEqual({
			text: 'Complete payment to proceed',
			isWaiting: false,
		});
	});

	it('returns payment failed message', () => {
		const info = getNextStepInfo('requested', 'failed');
		expect(info?.text).toContain('Payment failed');
		expect(info?.isWaiting).toBe(false);
	});

	it('returns waiting message for succeeded payment', () => {
		const info = getNextStepInfo('requested', 'succeeded');
		expect(info?.text).toContain('Waiting for provider');
		expect(info?.isWaiting).toBe(true);
	});

	it('returns provisioning message', () => {
		const info = getNextStepInfo('provisioning');
		expect(info?.text).toContain('setting up');
		expect(info?.isWaiting).toBe(true);
	});

	it('returns ready message for active', () => {
		const info = getNextStepInfo('active');
		expect(info?.text).toContain('ready');
		expect(info?.isWaiting).toBe(false);
	});

	it('returns ready message for provisioned', () => {
		const info = getNextStepInfo('provisioned');
		expect(info?.text).toContain('ready');
		expect(info?.isWaiting).toBe(false);
	});

	it('returns rejected message', () => {
		const info = getNextStepInfo('rejected');
		expect(info?.text).toContain('rejected');
		expect(info?.isWaiting).toBe(false);
	});

	it('returns failed message', () => {
		const info = getNextStepInfo('failed');
		expect(info?.text).toContain('failed');
		expect(info?.isWaiting).toBe(false);
	});

	it('returns null for cancelled', () => {
		expect(getNextStepInfo('cancelled')).toBeNull();
	});
});

describe('formatElapsed', () => {
	it('formats "just now" for under 1 minute', () => {
		expect(formatElapsed(0)).toBe('just now');
		expect(formatElapsed(0.5)).toBe('just now');
		expect(formatElapsed(0.99)).toBe('just now');
	});

	it('formats minutes for under 1 hour', () => {
		expect(formatElapsed(1)).toBe('1m');
		expect(formatElapsed(5)).toBe('5m');
		expect(formatElapsed(59)).toBe('59m');
		expect(formatElapsed(59.9)).toBe('59m');
	});

	it('formats hours and minutes', () => {
		expect(formatElapsed(60)).toBe('1h');
		expect(formatElapsed(61)).toBe('1h 1m');
		expect(formatElapsed(90)).toBe('1h 30m');
		expect(formatElapsed(125)).toBe('2h 5m');
		expect(formatElapsed(1440)).toBe('24h');
	});
});

describe('stageElapsedMinutes', () => {
	it('calculates elapsed time from created_at', () => {
		const nowNs = Date.now() * 1_000_000;
		const fiveMinAgoNs = nowNs - (5 * 60 * 1_000_000_000);
		const elapsed = stageElapsedMinutes(fiveMinAgoNs);
		expect(elapsed).toBeGreaterThanOrEqual(4.9);
		expect(elapsed).toBeLessThanOrEqual(5.1);
	});

	it('uses status_updated_at when provided', () => {
		const nowNs = Date.now() * 1_000_000;
		const oldNs = nowNs - (60 * 60 * 1_000_000_000);
		const recentNs = nowNs - (10 * 60 * 1_000_000_000);
		const elapsed = stageElapsedMinutes(oldNs, recentNs);
		expect(elapsed).toBeGreaterThanOrEqual(9.9);
		expect(elapsed).toBeLessThanOrEqual(10.1);
	});
});

describe('isStageOverdue', () => {
	it('returns false when timing has max 0', () => {
		const nowNs = Date.now() * 1_000_000;
		expect(isStageOverdue('active', undefined, nowNs - (100 * 60 * 1_000_000_000))).toBe(false);
	});

	it('returns false when within expected time', () => {
		const nowNs = Date.now() * 1_000_000;
		const twoMinAgoNs = nowNs - (2 * 60 * 1_000_000_000);
		expect(isStageOverdue('provisioning', undefined, twoMinAgoNs)).toBe(false);
	});

	it('returns true when exceeded max time', () => {
		const nowNs = Date.now() * 1_000_000;
		const thirtyMinAgoNs = nowNs - (30 * 60 * 1_000_000_000);
		expect(isStageOverdue('provisioning', undefined, thirtyMinAgoNs)).toBe(true);
	});

	it('returns false for terminal states', () => {
		const nowNs = Date.now() * 1_000_000;
		expect(isStageOverdue('cancelled', undefined, nowNs)).toBe(false);
		expect(isStageOverdue('rejected', undefined, nowNs)).toBe(false);
	});

	it('uses status_updated_at for timing', () => {
		const nowNs = Date.now() * 1_000_000;
		const oldNs = nowNs - (60 * 60 * 1_000_000_000);
		const recentNs = nowNs - (25 * 60 * 1_000_000_000);
		expect(isStageOverdue('provisioning', undefined, oldNs, recentNs)).toBe(true);
	});
});

describe('getStageTimingWithProvider', () => {
	it('returns static timing when no provider metrics available', () => {
		const timing = getStageTimingWithProvider('requested', 'pending', null);
		expect(timing).not.toBeNull();
		expect(timing?.max).toBe(5);
	});

	it('uses avgResponseTimeHours for requested+succeeded state', () => {
		const providerTiming: ProviderTimingEstimate = {
			avgResponseTimeHours: 2,
			timeToDeliveryHours: null,
		};
		const timing = getStageTimingWithProvider('requested', 'succeeded', providerTiming);
		expect(timing).not.toBeNull();
		expect(timing?.max).toBe(120); // 2 hours = 120 minutes
		expect(timing?.label).toContain('avg');
	});

	it('uses avgResponseTimeHours for pending state', () => {
		const providerTiming: ProviderTimingEstimate = {
			avgResponseTimeHours: 0.5, // 30 minutes
			timeToDeliveryHours: null,
		};
		const timing = getStageTimingWithProvider('pending', undefined, providerTiming);
		expect(timing).not.toBeNull();
		expect(timing?.max).toBe(30);
		expect(timing?.label).toContain('30min');
	});

	it('uses timeToDeliveryHours for accepted state', () => {
		const providerTiming: ProviderTimingEstimate = {
			avgResponseTimeHours: null,
			timeToDeliveryHours: 0.25, // 15 minutes
		};
		const timing = getStageTimingWithProvider('accepted', undefined, providerTiming);
		expect(timing).not.toBeNull();
		expect(timing?.max).toBe(15);
	});

	it('uses timeToDeliveryHours for provisioning state', () => {
		const providerTiming: ProviderTimingEstimate = {
			avgResponseTimeHours: null,
			timeToDeliveryHours: 0.5, // 30 minutes
		};
		const timing = getStageTimingWithProvider('provisioning', undefined, providerTiming);
		expect(timing).not.toBeNull();
		expect(timing?.label).toContain('VM setup');
	});

	it('falls back to static timing when provider metrics are null', () => {
		const providerTiming: ProviderTimingEstimate = {
			avgResponseTimeHours: null,
			timeToDeliveryHours: null,
		};
		const timing = getStageTimingWithProvider('requested', 'succeeded', providerTiming);
		expect(timing).not.toBeNull();
		expect(timing?.max).toBe(1440); // Default 24 hours
	});

	it('returns null for terminal states', () => {
		expect(getStageTimingWithProvider('cancelled', undefined, null)).toBeNull();
		expect(getStageTimingWithProvider('rejected', undefined, null)).toBeNull();
	});
});

describe('getStageExplanation', () => {
	it('returns explanation for payment pending', () => {
		const explanation = getStageExplanation('requested', 'pending');
		expect(explanation).not.toBeNull();
		expect(explanation).toContain('payment');
	});

	it('returns explanation for payment failed', () => {
		const explanation = getStageExplanation('requested', 'failed');
		expect(explanation).not.toBeNull();
		expect(explanation?.toLowerCase()).toContain('could not be completed');
	});

	it('returns explanation for payment succeeded', () => {
		const explanation = getStageExplanation('requested', 'succeeded');
		expect(explanation).not.toBeNull();
		expect(explanation).toContain('Payment received');
	});

	it('returns explanation for pending state', () => {
		const explanation = getStageExplanation('pending');
		expect(explanation).not.toBeNull();
		expect(explanation).toContain('review');
	});

	it('returns explanation for accepted state', () => {
		const explanation = getStageExplanation('accepted');
		expect(explanation).not.toBeNull();
		expect(explanation).toContain('accepted');
	});

	it('returns explanation for provisioning state', () => {
		const explanation = getStageExplanation('provisioning');
		expect(explanation).not.toBeNull();
		expect(explanation).toContain('VM');
	});

	it('returns explanation for active state', () => {
		const explanation = getStageExplanation('active');
		expect(explanation).not.toBeNull();
		expect(explanation).toContain('ready');
	});

	it('returns explanation for rejected state', () => {
		const explanation = getStageExplanation('rejected');
		expect(explanation).not.toBeNull();
		expect(explanation).toContain('declined');
	});

	it('returns explanation for failed state', () => {
		const explanation = getStageExplanation('failed');
		expect(explanation).not.toBeNull();
		expect(explanation).toContain('provisioning');
	});

	it('returns explanation for cancelled state', () => {
		const explanation = getStageExplanation('cancelled');
		expect(explanation).not.toBeNull();
		expect(explanation).toContain('cancelled');
	});
});

describe('LIFECYCLE_STAGES descriptions', () => {
	it('all stages have descriptions', () => {
		for (const stage of LIFECYCLE_STAGES) {
			expect(stage.description).toBeTruthy();
			expect(stage.description.length).toBeGreaterThan(10);
		}
	});
});

describe('getProgressSummary', () => {
	it('returns summary for payment pending', () => {
		const summary = getProgressSummary('requested', 'pending');
		expect(summary).not.toBeNull();
		expect(summary?.title).toBe('Processing Payment');
		expect(summary?.showSpinner).toBe(true);
		expect(summary?.estimatedTime).toContain('min');
	});

	it('returns summary for payment failed', () => {
		const summary = getProgressSummary('requested', 'failed');
		expect(summary).not.toBeNull();
		expect(summary?.title).toBe('Payment Failed');
		expect(summary?.showSpinner).toBe(false);
		expect(summary?.estimatedTime).toBeNull();
	});

	it('returns summary for payment succeeded (waiting for provider)', () => {
		const summary = getProgressSummary('requested', 'succeeded');
		expect(summary).not.toBeNull();
		expect(summary?.title).toBe('Waiting for Provider');
		expect(summary?.showSpinner).toBe(true);
		expect(summary?.estimatedTime).toContain('hours');
	});

	it('returns summary for pending state', () => {
		const summary = getProgressSummary('pending');
		expect(summary).not.toBeNull();
		expect(summary?.title).toBe('Waiting for Provider');
		expect(summary?.showSpinner).toBe(true);
	});

	it('returns summary for accepted state', () => {
		const summary = getProgressSummary('accepted');
		expect(summary).not.toBeNull();
		expect(summary?.title).toBe('Queued for Provisioning');
		expect(summary?.showSpinner).toBe(true);
		expect(summary?.estimatedTime).toContain('min');
	});

	it('returns summary for provisioning state', () => {
		const summary = getProgressSummary('provisioning');
		expect(summary).not.toBeNull();
		expect(summary?.title).toBe('Setting Up Your VM');
		expect(summary?.showSpinner).toBe(true);
		expect(summary?.estimatedTime).toContain('min');
	});

	it('returns summary for provisioned state', () => {
		const summary = getProgressSummary('provisioned');
		expect(summary).not.toBeNull();
		expect(summary?.title).toBe('Almost Ready');
		expect(summary?.showSpinner).toBe(true);
	});

	it('returns summary for active state', () => {
		const summary = getProgressSummary('active');
		expect(summary).not.toBeNull();
		expect(summary?.title).toBe('Ready to Use');
		expect(summary?.showSpinner).toBe(false);
		expect(summary?.estimatedTime).toBeNull();
	});

	it('returns summary for rejected state', () => {
		const summary = getProgressSummary('rejected');
		expect(summary).not.toBeNull();
		expect(summary?.title).toBe('Request Rejected');
		expect(summary?.showSpinner).toBe(false);
	});

	it('returns summary for failed state', () => {
		const summary = getProgressSummary('failed');
		expect(summary).not.toBeNull();
		expect(summary?.title).toBe('Provisioning Failed');
		expect(summary?.showSpinner).toBe(false);
	});

	it('returns summary for cancelled state', () => {
		const summary = getProgressSummary('cancelled');
		expect(summary).not.toBeNull();
		expect(summary?.title).toBe('Cancelled');
		expect(summary?.showSpinner).toBe(false);
	});

	it('handles case-insensitivity', () => {
		const summary = getProgressSummary('PROVISIONING');
		expect(summary).not.toBeNull();
		expect(summary?.title).toBe('Setting Up Your VM');
	});
});
