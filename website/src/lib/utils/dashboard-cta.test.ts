import { describe, expect, it } from 'vitest';
import {
	assertDashboardPeerCtaHeights,
	type DashboardCtaKind,
} from './dashboard-cta';

describe('assertDashboardPeerCtaHeights', () => {
	it('enforces <=2px height difference for marketplace active filter peers', () => {
		const result = assertDashboardPeerCtaHeights([
			{ kind: 'marketplace-active-filter-chip' },
			{ kind: 'marketplace-active-filter-clear' },
		]);

		expect(result.pass).toBe(true);
		expect(result.deltaPx).toBeLessThanOrEqual(2);
	});

	it('enforces <=2px height difference for rentals contract card action peers', () => {
		const result = assertDashboardPeerCtaHeights([
			{ kind: 'rentals-contract-action-primary' },
			{ kind: 'rentals-contract-action-secondary' },
			{ kind: 'rentals-contract-action-warning' },
		]);

		expect(result.pass).toBe(true);
		expect(result.deltaPx).toBeLessThanOrEqual(2);
	});

	it('ignores intentionally icon-only controls in peer checks', () => {
		const result = assertDashboardPeerCtaHeights([
			{ kind: 'rentals-pending-guidance-view' },
			{ kind: 'rentals-pending-guidance-dismiss', iconOnly: true },
		]);

		expect(result.pass).toBe(true);
		expect(result.measuredKinds).toEqual<DashboardCtaKind[]>(['rentals-pending-guidance-view']);
	});

	it('fails when peer CTA heights differ by more than threshold', () => {
		const result = assertDashboardPeerCtaHeights([
			{ kind: 'rentals-empty-state-cta' },
			{ kind: 'rentals-contract-action-primary' },
		]);

		expect(result.pass).toBe(false);
		expect(result.deltaPx).toBeGreaterThan(2);
	});
});
