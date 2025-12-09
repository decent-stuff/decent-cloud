import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
	createRentalRequest,
	getPendingProviderRequests,
	respondToRentalRequest,
	updateProvisioningStatus,
	cancelRentalRequest,
	type ProviderRentalResponseParams,
	type ProvisioningStatusUpdateParams,
	type RentalRequestParams
} from './api';

// Mock fetch
globalThis.fetch = vi.fn() as typeof fetch;

describe('createRentalRequest', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('sends POST request with correct parameters', async () => {
		const params: RentalRequestParams = {
			offering_db_id: 123,
			ssh_pubkey: 'ssh-ed25519 AAAA...',
			contact_method: 'email:test@example.com',
			request_memo: 'Please provision quickly',
			duration_hours: 720
		};

		const mockHeaders = {
			'X-Public-Key': 'test-pubkey',
			'X-Signature': 'test-signature',
			'X-Timestamp': '1234567890000000000',
			'X-Nonce': 'test-nonce-uuid',
			'Content-Type': 'application/json'
		};

		const mockResponse = {
			contractId: 'abc123def456',
			message: 'Rental request created successfully'
		};

		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: mockResponse
			})
		} as Response);

		const result = await createRentalRequest(params, mockHeaders);

		expect(fetch).toHaveBeenCalledWith(
			expect.stringContaining('/api/v1/contracts'),
			expect.objectContaining({
				method: 'POST',
				headers: mockHeaders,
				body: JSON.stringify(params)
			})
		);

		expect(result).toEqual(mockResponse);
		expect(result.contractId).toBe('abc123def456');
	});

	it('handles minimal parameters (only offering_db_id required)', async () => {
		const params: RentalRequestParams = {
			offering_db_id: 456
		};

		const mockHeaders = {
			'X-Public-Key': 'test-pubkey',
			'X-Signature': 'test-signature',
			'X-Timestamp': '1234567890000000000',
			'X-Nonce': 'test-nonce-uuid',
			'Content-Type': 'application/json'
		};

		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: {
					contractId: 'xyz789',
					message: 'Request created'
				}
			})
		} as Response);

		const result = await createRentalRequest(params, mockHeaders);

		expect(result.contractId).toBe('xyz789');
		expect(fetch).toHaveBeenCalledWith(
			expect.anything(),
			expect.objectContaining({
				body: JSON.stringify(params)
			})
		);
	});

	it('throws error when API returns non-ok response', async () => {
		const params: RentalRequestParams = {
			offering_db_id: 789
		};

		const mockHeaders = {
			'X-Public-Key': 'test-pubkey',
			'X-Signature': 'test-signature',
			'X-Timestamp': '1234567890000000000',
			'X-Nonce': 'test-nonce-uuid',
			'Content-Type': 'application/json'
		};

		vi.mocked(fetch).mockResolvedValue({
			ok: false,
			status: 400,
			statusText: 'Bad Request',
			text: async () => 'Offering not found'
		} as Response);

		await expect(createRentalRequest(params, mockHeaders)).rejects.toThrow(
			'Failed to create rental request'
		);
	});

	it('throws error when API returns success: false', async () => {
		const params: RentalRequestParams = {
			offering_db_id: 999
		};

		const mockHeaders = {
			'X-Public-Key': 'test-pubkey',
			'X-Signature': 'test-signature',
			'X-Timestamp': '1234567890000000000',
			'X-Nonce': 'test-nonce-uuid',
			'Content-Type': 'application/json'
		};

		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: false,
				error: 'Unauthorized: only provider can access this resource'
			})
		} as Response);

		await expect(createRentalRequest(params, mockHeaders)).rejects.toThrow(
			'Unauthorized: only provider can access this resource'
		);
	});

	it('throws error when API returns no data', async () => {
		const params: RentalRequestParams = {
			offering_db_id: 111
		};

		const mockHeaders = {
			'X-Public-Key': 'test-pubkey',
			'X-Signature': 'test-signature',
			'X-Timestamp': '1234567890000000000',
			'X-Nonce': 'test-nonce-uuid',
			'Content-Type': 'application/json'
		};

		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: null
			})
		} as Response);

		await expect(createRentalRequest(params, mockHeaders)).rejects.toThrow(
			'Rental request response did not include data'
		);
	});

	it('includes all optional parameters when provided', async () => {
		const params: RentalRequestParams = {
			offering_db_id: 555,
			ssh_pubkey: 'ssh-rsa AAAAB3NzaC1...',
			contact_method: 'matrix:@user:server.com',
			request_memo: 'Need GPU access for ML training',
			duration_hours: 168
		};

		const mockHeaders = {
			'X-Public-Key': 'test-pubkey',
			'X-Signature': 'test-signature',
			'X-Timestamp': '1234567890000000000',
			'X-Nonce': 'test-nonce-uuid',
			'Content-Type': 'application/json'
		};

		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: {
					contract_id: 'contract123',
					message: 'Success'
				}
			})
		} as Response);

		await createRentalRequest(params, mockHeaders);

		const fetchCall = vi.mocked(fetch).mock.calls[0];
		const body = JSON.parse(fetchCall[1]?.body as string);

		expect(body.ssh_pubkey).toBe('ssh-rsa AAAAB3NzaC1...');
		expect(body.contact_method).toBe('matrix:@user:server.com');
		expect(body.request_memo).toBe('Need GPU access for ML training');
		expect(body.duration_hours).toBe(168);
	});

	it('uses correct API endpoint', async () => {
		const params: RentalRequestParams = {
			offering_db_id: 222
		};

		const mockHeaders = {
			'X-Public-Key': 'test-pubkey',
			'X-Signature': 'test-signature',
			'X-Timestamp': '1234567890000000000',
			'X-Nonce': 'test-nonce-uuid',
			'Content-Type': 'application/json'
		};

		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: {
					contract_id: 'test',
					message: 'ok'
				}
			})
		} as Response);

		await createRentalRequest(params, mockHeaders);

		expect(fetch).toHaveBeenCalledWith(
			expect.stringMatching(/\/api\/v1\/contracts$/),
			expect.anything()
		);
	});
});

describe('getPendingProviderRequests', () => {
	const mockHeaders = {
		'X-Public-Key': 'provider-key',
		'X-Signature': 'sig',
		'X-Timestamp': '123',
		'X-Nonce': 'test-nonce-uuid',
		'Content-Type': 'application/json'
	};

	it('returns normalized contracts when API succeeds', async () => {
		// API returns hex strings from backend (via sqlx lower(hex(...)))
		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: [
					{
						contract_id: '0102',
						requester_pubkey: '0a0b',
						provider_pubkey: '0c0d',
						requester_ssh_pubkey: 'ssh-key',
						requester_contact: 'email:user@example.com',
						provider_pubkey_hex: 'ignored',
						offering_id: 'offer-1',
						payment_amount_e9s: 1_000_000_000,
						request_memo: 'memo',
						created_at_ns: 1,
						status: 'pending'
					}
				]
			})
		} as Response);

		const result = await getPendingProviderRequests(mockHeaders);
		expect(result).toHaveLength(1);
		expect(result[0].contract_id).toBe('0102');
		expect(result[0].requester_pubkey).toBe('0a0b');
		expect(fetch).toHaveBeenCalledWith(
			expect.stringContaining('/api/v1/provider/rental-requests/pending'),
			expect.objectContaining({ method: 'GET' })
		);
	});

	it('throws when API indicates failure', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: false,
				error: 'Not authorized'
			})
		} as Response);

		await expect(getPendingProviderRequests(mockHeaders)).rejects.toThrow('Not authorized');
	});
});

describe('respondToRentalRequest', () => {
	const mockHeaders = {
		'X-Public-Key': 'provider-key',
		'X-Signature': 'sig',
		'X-Timestamp': '123',
		'X-Nonce': 'test-nonce-uuid',
		'Content-Type': 'application/json'
	};

	it('sends provider response payload', async () => {
		const params: ProviderRentalResponseParams = {
			accept: true,
			memo: 'Ready'
		};

		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: 'Contract accepted'
			})
		} as Response);

		const message = await respondToRentalRequest('abcd', params, mockHeaders);
		expect(message).toBe('Contract accepted');

		expect(fetch).toHaveBeenCalledWith(
			expect.stringContaining('/api/v1/provider/rental-requests/abcd/respond'),
			expect.objectContaining({
				method: 'POST',
				body: JSON.stringify(params)
			})
		);
	});

	it('throws when HTTP layer fails', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: false,
			status: 403,
			statusText: 'Forbidden',
			text: async () => 'Forbidden'
		} as Response);

		await expect(
			respondToRentalRequest(
				'abcd',
				{
					accept: false
				},
				mockHeaders
			)
		).rejects.toThrow('Failed to respond to rental request');
	});
});

describe('updateProvisioningStatus', () => {
	const mockHeaders = {
		'X-Public-Key': 'provider-key',
		'X-Signature': 'sig',
		'X-Timestamp': '123',
		'X-Nonce': 'test-nonce-uuid',
		'Content-Type': 'application/json'
	};

	it('updates provisioning status successfully', async () => {
		const params: ProvisioningStatusUpdateParams = {
			status: 'provisioning',
			instanceDetails: 'Starting install'
		};

		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: 'Provisioning status updated'
			})
		} as Response);

		const message = await updateProvisioningStatus('abcd', params, mockHeaders);
		expect(message).toBe('Provisioning status updated');

		expect(fetch).toHaveBeenCalledWith(
			expect.stringContaining('/api/v1/provider/rental-requests/abcd/provisioning'),
			expect.objectContaining({
				method: 'PUT',
				body: JSON.stringify(params)
			})
		);
	});

	it('throws when API returns no message', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: null
			})
		} as Response);

		await expect(
			updateProvisioningStatus(
				'abcd',
				{
					status: 'provisioned',
					instanceDetails: 'ip:1.2.3.4'
				},
				mockHeaders
			)
		).rejects.toThrow('Provisioning status response did not include confirmation message');
	});
});

describe('cancelRentalRequest', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('sends PUT request with correct parameters', async () => {
		const mockHeaders = {
			'X-Public-Key': 'test-pubkey',
			'X-Signature': 'test-signature',
			'X-Timestamp': '1234567890000000000',
			'X-Nonce': 'test-nonce-uuid',
			'Content-Type': 'application/json'
		};

		const mockResponse = 'Rental request cancelled successfully';

		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: mockResponse
			})
		} as Response);

		const result = await cancelRentalRequest('abc123', { memo: 'User cancellation' }, mockHeaders);

		expect(vi.mocked(fetch)).toHaveBeenCalledWith(
			expect.stringContaining('/api/v1/contracts/abc123/cancel'),
			expect.objectContaining({
				method: 'PUT',
				headers: mockHeaders,
				body: JSON.stringify({ memo: 'User cancellation' })
			})
		);

		expect(result).toBe(mockResponse);
	});

	it('handles API errors correctly', async () => {
		const mockHeaders = {
			'X-Public-Key': 'test-pubkey',
			'X-Signature': 'test-signature',
			'X-Timestamp': '1234567890000000000',
			'X-Nonce': 'test-nonce-uuid',
			'Content-Type': 'application/json'
		};

		vi.mocked(fetch).mockResolvedValue({
			ok: false,
			status: 404,
			statusText: 'Not Found',
			json: async () => ({
				success: false,
				error: 'Contract not found'
			}),
			text: async () => 'Not Found'
		} as Response);

		await expect(
			cancelRentalRequest('nonexistent', { memo: 'Test' }, mockHeaders)
		).rejects.toThrow('Failed to cancel rental request: 404 Not Found');
	});

	it('handles missing data in response', async () => {
		const mockHeaders = {
			'X-Public-Key': 'test-pubkey',
			'X-Signature': 'test-signature',
			'X-Timestamp': '1234567890000000000',
			'X-Nonce': 'test-nonce-uuid',
			'Content-Type': 'application/json'
		};

		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: null
			})
		} as Response);

		await expect(
			cancelRentalRequest('abc123', { memo: 'Test' }, mockHeaders)
		).rejects.toThrow('Cancel rental request response did not include confirmation message');
	});
});
