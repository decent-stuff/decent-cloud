import { describe, it, expect } from 'vitest';
import { formatEventType, getEventIcon, formatEventActor } from './contract-events';

describe('formatEventType', () => {
	it('formats status_change', () => {
		expect(formatEventType('status_change')).toBe('Status Changed');
	});

	it('formats password_reset', () => {
		expect(formatEventType('password_reset')).toBe('Password Reset');
	});

	it('formats ssh_key_rotation', () => {
		expect(formatEventType('ssh_key_rotation')).toBe('SSH Key Rotation Requested');
	});

	it('formats ssh_key_rotation_complete', () => {
		expect(formatEventType('ssh_key_rotation_complete')).toBe('SSH Key Rotation Complete');
	});

	it('formats extension', () => {
		expect(formatEventType('extension')).toBe('Contract Extended');
	});

	it('formats payment_confirmed', () => {
		expect(formatEventType('payment_confirmed')).toBe('Payment Confirmed');
	});

	it('formats provisioned', () => {
		expect(formatEventType('provisioned')).toBe('Provisioned');
	});

	it('capitalizes words for unknown event types', () => {
		expect(formatEventType('some_unknown_event')).toBe('Some Unknown Event');
	});

	it('handles single-word unknown event type', () => {
		expect(formatEventType('created')).toBe('Created');
	});
});

describe('getEventIcon', () => {
	it('returns refresh for status_change', () => {
		expect(getEventIcon('status_change')).toBe('refresh');
	});

	it('returns key for password_reset', () => {
		expect(getEventIcon('password_reset')).toBe('key');
	});

	it('returns key for ssh_key_rotation', () => {
		expect(getEventIcon('ssh_key_rotation')).toBe('key');
	});

	it('returns check for ssh_key_rotation_complete', () => {
		expect(getEventIcon('ssh_key_rotation_complete')).toBe('check');
	});

	it('returns clock for extension', () => {
		expect(getEventIcon('extension')).toBe('clock');
	});

	it('returns check for payment_confirmed', () => {
		expect(getEventIcon('payment_confirmed')).toBe('check');
	});

	it('returns server for provisioned', () => {
		expect(getEventIcon('provisioned')).toBe('server');
	});

	it('returns file for unknown event types', () => {
		expect(getEventIcon('something_else')).toBe('file');
	});
});

describe('formatEventActor', () => {
	it('formats provider', () => {
		expect(formatEventActor('provider')).toBe('Provider');
	});

	it('formats tenant', () => {
		expect(formatEventActor('tenant')).toBe('Tenant');
	});

	it('formats system', () => {
		expect(formatEventActor('system')).toBe('System');
	});

	it('capitalizes other actors', () => {
		expect(formatEventActor('admin')).toBe('Admin');
	});

	it('handles multi-word unknown actors by capitalizing first character', () => {
		expect(formatEventActor('custom_actor')).toBe('Custom_actor');
	});
});
