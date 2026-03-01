import { describe, it, expect } from 'vitest';
import { buildContractEventsUrl, parseContractEvent } from './contract-sse';
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
