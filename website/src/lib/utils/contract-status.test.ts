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
});
