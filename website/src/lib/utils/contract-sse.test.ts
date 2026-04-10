import { describe, it, expect } from 'vitest';
import { buildContractEventsUrl, parseContractEvent, buildPasswordResetEventsUrl, parsePasswordResetEvent, parseSshKeyRotationEvent } from './contract-sse';
import type { SignedRequestHeaders } from '$lib/types/generated/SignedRequestHeaders';

describe('buildContractEventsUrl', () => {
	it('builds correct URL with pubkey and apiUrl', () => {
		const url = buildContractEventsUrl('abc123', 'https://api.example.com');
		expect(url).toBe('https://api.example.com/api/v1/users/abc123/contract-events');
	});

	it('works with empty apiUrl (relative URL)', () => {
		const url = buildContractEventsUrl('deadbeef', '');
		expect(url).toBe('/api/v1/users/deadbeef/contract-events');
	});

	it('includes auth headers as query params when provided', () => {
		const headers: SignedRequestHeaders = {
			'X-Public-Key': 'pubkey123',
			'X-Signature': 'sig456',
			'X-Timestamp': '789',
			'X-Nonce': 'nonce000',
			'Content-Type': 'application/json'
		};
		const url = buildContractEventsUrl('abc123', 'https://api.example.com', headers);
		expect(url).toContain('https://api.example.com/api/v1/users/abc123/contract-events?');
		expect(url).toContain('pubkey=pubkey123');
		expect(url).toContain('signature=sig456');
		expect(url).toContain('timestamp=789');
		expect(url).toContain('nonce=nonce000');
	});

	it('URL-encodes special characters in auth params', () => {
		const headers: SignedRequestHeaders = {
			'X-Public-Key': 'abc+def/ghi==',
			'X-Signature': 'sig',
			'X-Timestamp': '123',
			'X-Nonce': 'nonce',
			'Content-Type': 'application/json'
		};
		const url = buildContractEventsUrl('abc123', 'https://api.example.com', headers);
		expect(url).toContain('pubkey=abc%2Bdef%2Fghi%3D%3D');
	});
});

describe('parseContractEvent', () => {
	it('parses valid event with updated_at_ns', () => {
		const data = JSON.stringify({
			contract_id: 'abc123',
			status: 'active',
			updated_at_ns: 1700000000000000000
		});
		const event = parseContractEvent(data);
		expect(event.contract_id).toBe('abc123');
		expect(event.status).toBe('active');
		expect(event.updated_at_ns).toBe(1700000000000000000);
	});

	it('treats non-numeric updated_at_ns as undefined', () => {
		const data = JSON.stringify({
			contract_id: 'def456',
			status: 'pending',
			updated_at_ns: null
		});
		const event = parseContractEvent(data);
		expect(event.contract_id).toBe('def456');
		expect(event.status).toBe('pending');
		expect(event.updated_at_ns).toBeUndefined();
	});

	it('treats missing updated_at_ns as undefined', () => {
		const data = JSON.stringify({ contract_id: 'ghi789', status: 'provisioning' });
		const event = parseContractEvent(data);
		expect(event.updated_at_ns).toBeUndefined();
	});

	it('throws on malformed JSON', () => {
		expect(() => parseContractEvent('not json')).toThrow();
	});

	it('throws when contract_id is missing', () => {
		const data = JSON.stringify({ status: 'active', updated_at_ns: 0 });
		expect(() => parseContractEvent(data)).toThrow('Invalid contract event payload');
	});

	it('throws when status is missing', () => {
		const data = JSON.stringify({ contract_id: 'abc', updated_at_ns: 0 });
		expect(() => parseContractEvent(data)).toThrow('Invalid contract event payload');
	});

	it('throws on non-object payload', () => {
		expect(() => parseContractEvent('"just a string"')).toThrow('Invalid contract event payload');
	});
});

describe('buildPasswordResetEventsUrl', () => {
	it('builds correct URL with providerPubkey and apiUrl', () => {
		const headers: SignedRequestHeaders = {
			'X-Public-Key': 'pubkey123',
			'X-Signature': 'sig456',
			'X-Timestamp': '789',
			'X-Nonce': 'nonce000',
			'Content-Type': 'application/json'
		};
		const url = buildPasswordResetEventsUrl('abc123', 'https://api.example.com', headers);
		expect(url).toContain('https://api.example.com/api/v1/providers/abc123/password-reset-events?');
		expect(url).toContain('pubkey=pubkey123');
		expect(url).toContain('signature=sig456');
		expect(url).toContain('timestamp=789');
		expect(url).toContain('nonce=nonce000');
	});

	it('works with empty apiUrl (relative URL)', () => {
		const headers: SignedRequestHeaders = {
			'X-Public-Key': 'pubkey',
			'X-Signature': 'sig',
			'X-Timestamp': '123',
			'X-Nonce': 'nonce',
			'Content-Type': 'application/json'
		};
		const url = buildPasswordResetEventsUrl('deadbeef', '', headers);
		expect(url).toContain('/api/v1/providers/deadbeef/password-reset-events?');
	});

	it('URL-encodes special characters in auth params', () => {
		const headers: SignedRequestHeaders = {
			'X-Public-Key': 'abc+def/ghi==',
			'X-Signature': 'sig',
			'X-Timestamp': '123',
			'X-Nonce': 'nonce',
			'Content-Type': 'application/json'
		};
		const url = buildPasswordResetEventsUrl('abc123', 'https://api.example.com', headers);
		expect(url).toContain('pubkey=abc%2Bdef%2Fghi%3D%3D');
	});
});

describe('parsePasswordResetEvent', () => {
	it('parses valid event with count and contract_ids', () => {
		const data = JSON.stringify({
			count: 3,
			contract_ids: ['abc123', 'def456', 'ghi789']
		});
		const event = parsePasswordResetEvent(data);
		expect(event.count).toBe(3);
		expect(event.contract_ids).toEqual(['abc123', 'def456', 'ghi789']);
	});

	it('parses event with empty contract_ids array', () => {
		const data = JSON.stringify({
			count: 0,
			contract_ids: []
		});
		const event = parsePasswordResetEvent(data);
		expect(event.count).toBe(0);
		expect(event.contract_ids).toEqual([]);
	});

	it('throws on malformed JSON', () => {
		expect(() => parsePasswordResetEvent('not json')).toThrow();
	});

	it('throws when count is missing', () => {
		const data = JSON.stringify({ contract_ids: ['abc'] });
		expect(() => parsePasswordResetEvent(data)).toThrow('Invalid password reset event payload');
	});

	it('throws when contract_ids is missing', () => {
		const data = JSON.stringify({ count: 1 });
		expect(() => parsePasswordResetEvent(data)).toThrow('Invalid password reset event payload');
	});

	it('throws when count is not a number', () => {
		const data = JSON.stringify({ count: 'three', contract_ids: [] });
		expect(() => parsePasswordResetEvent(data)).toThrow('Invalid password reset event payload');
	});

	it('throws when contract_ids is not an array', () => {
		const data = JSON.stringify({ count: 1, contract_ids: 'not-an-array' });
		expect(() => parsePasswordResetEvent(data)).toThrow('Invalid password reset event payload');
	});

	it('throws on non-object payload', () => {
		expect(() => parsePasswordResetEvent('"just a string"')).toThrow('Invalid password reset event payload');
	});
});

describe('parseSshKeyRotationEvent', () => {
	it('parses valid ssh_key_rotation event', () => {
		const data = JSON.stringify({
			contract_id: 'abc123',
			created_at: 1700000000000000000,
			actor: 'tenant',
			details: null
		});
		const event = parseSshKeyRotationEvent(data);
		expect(event.contract_id).toBe('abc123');
		expect(event.created_at).toBe(1700000000000000000);
		expect(event.actor).toBe('tenant');
		expect(event.details).toBeNull();
	});

	it('parses valid ssh_key_rotation_complete event with details', () => {
		const data = JSON.stringify({
			contract_id: 'def456',
			created_at: 1700000001000000000,
			actor: 'provider',
			details: 'SSH key rotated to ssh-ed25519 AAA... by agent'
		});
		const event = parseSshKeyRotationEvent(data);
		expect(event.contract_id).toBe('def456');
		expect(event.created_at).toBe(1700000001000000000);
		expect(event.actor).toBe('provider');
		expect(event.details).toBe('SSH key rotated to ssh-ed25519 AAA... by agent');
	});

	it('defaults created_at to 0 when missing', () => {
		const data = JSON.stringify({
			contract_id: 'abc',
			actor: 'tenant'
		});
		const event = parseSshKeyRotationEvent(data);
		expect(event.created_at).toBe(0);
	});

	it('defaults details to null when not a string', () => {
		const data = JSON.stringify({
			contract_id: 'abc',
			actor: 'tenant',
			details: 42
		});
		const event = parseSshKeyRotationEvent(data);
		expect(event.details).toBeNull();
	});

	it('throws on malformed JSON', () => {
		expect(() => parseSshKeyRotationEvent('not json')).toThrow();
	});

	it('throws when contract_id is missing', () => {
		const data = JSON.stringify({ actor: 'tenant', created_at: 0 });
		expect(() => parseSshKeyRotationEvent(data)).toThrow('Invalid SSH key rotation event payload');
	});

	it('throws when actor is missing', () => {
		const data = JSON.stringify({ contract_id: 'abc', created_at: 0 });
		expect(() => parseSshKeyRotationEvent(data)).toThrow('Invalid SSH key rotation event payload');
	});

	it('throws on non-object payload', () => {
		expect(() => parseSshKeyRotationEvent('"just a string"')).toThrow('Invalid SSH key rotation event payload');
	});
});
