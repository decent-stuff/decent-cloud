import { describe, it, expect } from 'vitest';
import { getContractStatusBadge } from './contract-status';

describe('getContractStatusBadge', () => {
	it('returns known badges for core statuses', () => {
		const badge = getContractStatusBadge('accepted');
		expect(badge.text).toBe('Accepted');
		expect(badge.icon).toBe('🟢');
	});

	it('treats status names case-insensitively', () => {
		const badge = getContractStatusBadge('ProVisioned');
		expect(badge.text).toBe('Provisioned');
	});

	it('falls back to default badge for unknown statuses', () => {
		const badge = getContractStatusBadge('custom');
		expect(badge.text).toBe('custom');
		expect(badge.icon).toBe('⚪');
	});

	// Payment-aware status tests
	it('shows Awaiting Payment for requested + pending payment', () => {
		const badge = getContractStatusBadge('requested', 'pending');
		expect(badge.text).toBe('Awaiting Payment');
		expect(badge.icon).toBe('💳');
	});

	it('shows Payment Failed for requested + failed payment', () => {
		const badge = getContractStatusBadge('requested', 'failed');
		expect(badge.text).toBe('Payment Failed');
		expect(badge.icon).toBe('❌');
	});

	it('shows Pending Provider for requested + succeeded payment', () => {
		const badge = getContractStatusBadge('requested', 'succeeded');
		expect(badge.text).toBe('Pending Provider');
		expect(badge.icon).toBe('⏳');
	});

	it('ignores payment status for non-requested statuses', () => {
		const badge = getContractStatusBadge('accepted', 'pending');
		expect(badge.text).toBe('Accepted');
	});

	it('handles missing payment status gracefully', () => {
		const badge = getContractStatusBadge('requested');
		expect(badge.text).toBe('Pending Provider');
	});

	it('shows Failed badge for failed status', () => {
		const badge = getContractStatusBadge('failed');
		expect(badge.text).toBe('Failed');
		expect(badge.icon).toBe('❗');
		expect(badge.class).toContain('danger');
	});

	it('shows Rejected badge for rejected status', () => {
		const badge = getContractStatusBadge('rejected');
		expect(badge.text).toBe('Rejected');
		expect(badge.icon).toBe('🔴');
		expect(badge.class).toContain('danger');
	});

	it('treats failed status case-insensitively', () => {
		const badge = getContractStatusBadge('FAILED');
		expect(badge.text).toBe('Failed');
	});
});
