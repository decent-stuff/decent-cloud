import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { getPendingPasswordResets, getPendingSshKeyRotations, rotateSshKey } from './api';
import type { SignedRequestHeaders } from '$lib/types/generated/SignedRequestHeaders';

const mockHeaders: SignedRequestHeaders = {
	'Content-Type': 'application/json',
	'X-Public-Key': 'deadbeef',
	'X-Timestamp': '123456789',
	'X-Nonce': 'test-nonce-uuid',
	'X-Signature': 'test-signature'
};

describe('getPendingPasswordResets', () => {
	beforeEach(() => {
		globalThis.fetch = vi.fn();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('returns empty array when no pending resets', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: [] })
		} as Response);

		const result = await getPendingPasswordResets('abc123', mockHeaders);
		expect(result).toEqual([]);
	});

	it('returns contracts with pending password resets', async () => {
		const mockContracts = [
			{
				contract_id: 'contract1',
				status: 'active',
				provider_pubkey: 'abc123',
				requester_pubkey: 'beef',
				requester_ssh_pubkey: 'ssh-ed25519 AAAA...',
				requester_contact: 'user@example.com',
				offering_id: 'off-1',
				payment_amount_e9s: 1_000_000_000,
				request_memo: '',
				created_at_ns: 1_700_000_000_000_000_000,
				payment_method: 'stripe',
				payment_status: 'paid',
				currency: 'USD',
				auto_renew: false
			}
		];
		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: mockContracts })
		} as Response);

		const result = await getPendingPasswordResets('abc123', mockHeaders);
		expect(result).toHaveLength(1);
		expect(result[0].contract_id).toBe('contract1');
	});

	it('calls the correct endpoint with provider pubkey', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: [] })
		} as Response);

		await getPendingPasswordResets('deadbeef', mockHeaders);

		expect(fetch).toHaveBeenCalledWith(
			expect.stringContaining('/providers/deadbeef/contracts/pending-password-reset'),
			expect.objectContaining({ method: 'GET', headers: mockHeaders })
		);
	});

	it('throws on API error response', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({ success: false, error: 'Unauthorized' })
		} as Response);

		await expect(getPendingPasswordResets('abc123', mockHeaders)).rejects.toThrow('Unauthorized');
	});

	it('throws on HTTP error', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: false,
			status: 403,
			statusText: 'Forbidden'
		} as Response);

		await expect(getPendingPasswordResets('abc123', mockHeaders)).rejects.toThrow('403');
	});
});

describe('getPendingSshKeyRotations', () => {
	beforeEach(() => {
		globalThis.fetch = vi.fn();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('returns only contracts with pending SSH key rotations', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: [
					{
						contract_id: 'rotation-1',
						status: 'active',
						provider_pubkey: 'abc123',
						requester_pubkey: 'beef',
						requester_ssh_pubkey: 'ssh-ed25519 AAAA current',
						requester_contact: 'user@example.com',
						offering_id: 'off-1',
						payment_amount_e9s: 1_000_000_000,
						request_memo: '',
						created_at_ns: 1_700_000_000_000_000_000,
						payment_method: 'stripe',
						payment_status: 'paid',
						currency: 'USD',
						auto_renew: false,
						ssh_key_rotation_requested_at_ns: 1_700_000_100_000_000_000
					},
					{
						contract_id: 'stable-1',
						status: 'active',
						provider_pubkey: 'abc123',
						requester_pubkey: 'cafe',
						requester_ssh_pubkey: 'ssh-ed25519 AAAA stable',
						requester_contact: 'other@example.com',
						offering_id: 'off-2',
						payment_amount_e9s: 2_000_000_000,
						request_memo: '',
						created_at_ns: 1_700_000_000_000_000_000,
						payment_method: 'stripe',
						payment_status: 'paid',
						currency: 'USD',
						auto_renew: false
					}
				]
			})
		} as Response);

		const result = await getPendingSshKeyRotations('abc123', mockHeaders);
		expect(result).toHaveLength(1);
		expect(result[0].contract_id).toBe('rotation-1');
	});

	it('propagates provider contract fetch failures', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: false,
			status: 500,
			statusText: 'Server Error'
		} as Response);

		await expect(getPendingSshKeyRotations('abc123', mockHeaders)).rejects.toThrow(
			'Failed to fetch provider contracts: 500 Server Error'
		);
	});
});

describe('rotateSshKey', () => {
	beforeEach(() => {
		globalThis.fetch = vi.fn();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('posts the new SSH public key to the rotation endpoint', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: 'ok' })
		} as Response);

		await rotateSshKey('deadbeef', 'ssh-ed25519 AAAA new-key', mockHeaders);

		expect(fetch).toHaveBeenCalledWith(
			expect.stringContaining('/contracts/deadbeef/rotate-ssh-key'),
			expect.objectContaining({
				method: 'POST',
				headers: mockHeaders,
				body: JSON.stringify({ new_ssh_pubkey: 'ssh-ed25519 AAAA new-key' })
			})
		);
	});

	it('throws the API error message when rotation request fails', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({ success: false, error: 'Invalid SSH public key format' })
		} as Response);

		await expect(rotateSshKey('deadbeef', 'bad-key', mockHeaders)).rejects.toThrow(
			'Invalid SSH public key format'
		);
	});
});
