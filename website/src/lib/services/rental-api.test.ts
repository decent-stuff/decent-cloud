import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createRentalRequest, type RentalRequestParams } from './api';

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
			'Content-Type': 'application/json'
		};

		const mockResponse = {
			contract_id: 'abc123def456',
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
			expect.stringContaining('/api/v1/contracts/rental-request'),
			expect.objectContaining({
				method: 'POST',
				headers: mockHeaders,
				body: JSON.stringify(params)
			})
		);

		expect(result).toEqual(mockResponse);
		expect(result.contract_id).toBe('abc123def456');
	});

	it('handles minimal parameters (only offering_db_id required)', async () => {
		const params: RentalRequestParams = {
			offering_db_id: 456
		};

		const mockHeaders = {
			'X-Public-Key': 'test-pubkey',
			'X-Signature': 'test-signature',
			'X-Timestamp': '1234567890000000000',
			'Content-Type': 'application/json'
		};

		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({
				success: true,
				data: {
					contract_id: 'xyz789',
					message: 'Request created'
				}
			})
		} as Response);

		const result = await createRentalRequest(params, mockHeaders);

		expect(result.contract_id).toBe('xyz789');
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
			expect.stringMatching(/\/api\/v1\/contracts\/rental-request$/),
			expect.anything()
		);
	});
});
