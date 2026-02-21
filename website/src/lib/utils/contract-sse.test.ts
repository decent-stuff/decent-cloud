import { describe, it, expect } from 'vitest';
import { buildContractEventsUrl, parseContractEvent } from './contract-sse';

describe('buildContractEventsUrl', () => {
	it('builds correct URL with pubkey and apiUrl', () => {
		const url = buildContractEventsUrl('abc123', 'https://api.example.com');
		expect(url).toBe('https://api.example.com/api/v1/users/abc123/contract-events');
	});

	it('works with empty apiUrl (relative URL)', () => {
		const url = buildContractEventsUrl('deadbeef', '');
		expect(url).toBe('/api/v1/users/deadbeef/contract-events');
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
