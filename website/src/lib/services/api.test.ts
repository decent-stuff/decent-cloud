import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { fetchPlatformStats, searchOfferings, getActiveProviders, getProviderOfferings, hexEncode, getUserContracts, getProviderContracts, getProviderResponseMetrics } from './api';
import type { SignedRequestHeaders } from '$lib/types/generated/SignedRequestHeaders';

const sampleStats = {
	total_providers: 10,
	active_providers: 8,
	total_offerings: 5,
	total_contracts: 3,
	total_transfers: 12,
	total_volume_e9s: 1_500_000_000,
	validator_count_24h: 4,
	latest_block_timestamp_ns: 123_456_789,
	metadata: {
		'ledger:num_blocks': 42,
		'ledger:blocks_until_next_halving': 210_000,
		'ledger:current_block_rewards_e9s': 50_000_000_000,
		'ledger:current_block_validators': 3,
		'ledger:token_value_in_usd_e6': 1_000_000
	}
};

const sampleOfferings = [
	{
		id: 1,
		pubkey: [1, 2, 3, 4],
		offering_id: 'off-1',
		offer_name: 'Test VM',
		product_type: 'compute',
		currency: 'USD',
		monthly_price: 10.0,
		setup_fee: 0,
		visibility: 'public',
		billing_interval: 'monthly',
		stock_status: 'in_stock',
		datacenter_country: 'US',
		datacenter_city: 'NYC',
		unmetered_bandwidth: false
	}
];

const sampleProviders = [
	{
		pubkey: [5, 6, 7, 8],
		name: 'Test Provider',
		api_version: '1.0',
		profile_version: '1.0',
		updated_at_ns: 123456789
	}
];

describe('fetchPlatformStats', () => {
	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('returns stats payload when API succeeds', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: sampleStats })
		});

		const stats = await fetchPlatformStats();

		expect(stats).toEqual(sampleStats);
		expect(globalThis.fetch).toHaveBeenCalledWith(expect.stringContaining('/api/v1/stats'));
	});

	it('throws when response is not ok', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: false,
			status: 500,
			statusText: 'error'
		});

		await expect(fetchPlatformStats()).rejects.toThrow('Failed to fetch platform stats');
	});

	it('throws when API reports failure', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: false, error: 'boom' })
		});

		await expect(fetchPlatformStats()).rejects.toThrow('boom');
	});

	it('throws when data is missing', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true })
		});

		await expect(fetchPlatformStats()).rejects.toThrow('did not include data');
	});
});

describe('searchOfferings', () => {
	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('returns offerings when API succeeds', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: sampleOfferings })
		});

		const offerings = await searchOfferings();

		expect(offerings).toHaveLength(1);
		expect(offerings[0].pubkey).toBe('01020304'); // Normalized to hex string
		expect(globalThis.fetch).toHaveBeenCalledWith(expect.stringContaining('/api/v1/offerings'));
	});

	it('normalizes pubkey from array to hex string', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: sampleOfferings })
		});

		const offerings = await searchOfferings();
		expect(typeof offerings[0].pubkey).toBe('string');
		expect(offerings[0].pubkey).toBe('01020304');
	});

	it('handles string pubkey', async () => {
		const offeringWithStringHash = [{ ...sampleOfferings[0], pubkey: 'abcd1234' }];
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: offeringWithStringHash })
		});

		const offerings = await searchOfferings();
		expect(offerings[0].pubkey).toBe('abcd1234');
	});

	it('passes query parameters correctly', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: [] })
		});

		await searchOfferings({ limit: 20, product_type: 'compute', in_stock_only: true });

		const callUrl = (globalThis.fetch as any).mock.calls[0][0];
		expect(callUrl).toContain('limit=20');
		expect(callUrl).toContain('product_type=compute');
		expect(callUrl).toContain('in_stock_only=true');
	});

	it('throws when API fails', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: false,
			status: 500,
			statusText: 'error'
		});

		await expect(searchOfferings()).rejects.toThrow('Failed to fetch offerings');
	});
});

describe('getActiveProviders', () => {
	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('returns providers when API succeeds', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: sampleProviders })
		});

		const providers = await getActiveProviders(1);

		expect(providers).toHaveLength(1);
		expect(providers[0].pubkey).toBe('05060708'); // Normalized to hex string
		expect(globalThis.fetch).toHaveBeenCalledWith(expect.stringContaining('/api/v1/providers/active/1'));
	});

	it('normalizes pubkey from array to hex string', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: sampleProviders })
		});

		const providers = await getActiveProviders(1);
		expect(typeof providers[0].pubkey).toBe('string');
		expect(providers[0].pubkey).toBe('05060708');
	});

	it('throws when API fails', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: false,
			status: 500,
			statusText: 'error'
		});

		await expect(getActiveProviders(1)).rejects.toThrow('Failed to fetch active providers');
	});
});

describe('getProviderOfferings', () => {
	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('returns offerings when API succeeds with hex string', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: sampleOfferings })
		});

		const offerings = await getProviderOfferings('abcd1234');

		expect(offerings).toHaveLength(1);
		expect(globalThis.fetch).toHaveBeenCalledWith(expect.stringContaining('/api/v1/providers/abcd1234/offerings'));
	});

	it('converts Uint8Array to hex string', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: sampleOfferings })
		});

		const pubkey = new Uint8Array([0xab, 0xcd, 0x12, 0x34]);
		await getProviderOfferings(pubkey);

		expect(globalThis.fetch).toHaveBeenCalledWith(expect.stringContaining('/api/v1/providers/abcd1234/offerings'));
	});

	it('normalizes pubkey in response', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: sampleOfferings })
		});

		const offerings = await getProviderOfferings('test');
		expect(typeof offerings[0].pubkey).toBe('string');
		expect(offerings[0].pubkey).toBe('01020304');
	});

	it('throws when API fails', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: false,
			status: 500,
			statusText: 'error'
		});

		await expect(getProviderOfferings('test')).rejects.toThrow('Failed to fetch provider offerings');
	});
});

describe('hexEncode', () => {
	it('encodes Uint8Array to hex string', () => {
		const bytes = new Uint8Array([0xab, 0xcd, 0x12, 0x34]);
		expect(hexEncode(bytes)).toBe('abcd1234');
	});

	it('encodes number array to hex string', () => {
		const bytes = [0xab, 0xcd, 0x12, 0x34];
		expect(hexEncode(bytes)).toBe('abcd1234');
	});

	it('pads single digit hex values', () => {
		const bytes = new Uint8Array([0x0a, 0x0b]);
		expect(hexEncode(bytes)).toBe('0a0b');
	});
});

describe('getUserContracts', () => {
	// API returns hex strings from backend (via sqlx lower(hex(...)))
	const sampleContracts = [
		{
			contract_id: '01020304',
			requester_pubkey: '05060708',
			provider_pubkey: '090a0b0c',
			requester_ssh_pubkey: 'ssh-ed25519 AAAA...',
			requester_contact: 'user@example.com',
			offering_id: 'off-123',
			payment_amount_e9s: 1000000000,
			request_memo: 'Test rental',
			created_at_ns: 1234567890000000,
			status: 'requested'
		}
	];

	const mockHeaders: SignedRequestHeaders = {
		'Content-Type': 'application/json',
		'X-Public-Key': 'test-pubkey',
		'X-Timestamp': '123456789',
		'X-Nonce': 'test-nonce-uuid',
		'X-Signature': 'test-signature'
	};

	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('fetches and normalizes user contracts successfully', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: sampleContracts })
		});

		const contracts = await getUserContracts(mockHeaders);

		expect(contracts).toHaveLength(1);
		expect(contracts[0].contract_id).toBe('01020304');
		expect(contracts[0].requester_pubkey).toBe('05060708');
		expect(contracts[0].provider_pubkey).toBe('090a0b0c');
		expect(globalThis.fetch).toHaveBeenCalledWith(
			expect.stringContaining('/api/v1/users/test-pubkey/contracts'),
			expect.objectContaining({
				method: 'GET',
				headers: mockHeaders
			})
		);
	});

	it('handles string pubkey hashes', async () => {
		const contractsWithStringHashes = [
			{
				...sampleContracts[0],
				contract_id: 'abc123',
				requester_pubkey: 'def456',
				provider_pubkey: 'ghi789'
			}
		];

		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: contractsWithStringHashes })
		});

		const contracts = await getUserContracts(mockHeaders);

		expect(contracts[0].contract_id).toBe('abc123');
		expect(contracts[0].requester_pubkey).toBe('def456');
		expect(contracts[0].provider_pubkey).toBe('ghi789');
	});

	it('returns empty array when no contracts exist', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: [] })
		});

		const contracts = await getUserContracts(mockHeaders);

		expect(contracts).toEqual([]);
	});

	it('throws when API response is not ok', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: false,
			status: 401,
			statusText: 'Unauthorized'
		});

		await expect(getUserContracts(mockHeaders)).rejects.toThrow(
			'Failed to fetch user contracts: 401 Unauthorized'
		);
	});

	it('throws when API reports failure', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: false, error: 'Database error' })
		});

		await expect(getUserContracts(mockHeaders)).rejects.toThrow('Database error');
	});
});

describe('getProviderContracts', () => {
	const providerHex = 'abcd1234';
	const providerHeaders: SignedRequestHeaders = {
		'X-Public-Key': providerHex,
		'X-Signature': 'sig',
		'X-Timestamp': '123',
		'X-Nonce': 'test-nonce-uuid',
		'Content-Type': 'application/json'
	};

	beforeEach(() => {
		globalThis.fetch = vi.fn();
	});

	it('fetches provider contracts for the given pubkey', async () => {
		// API returns hex strings from backend
		const contracts = [
			{
				contract_id: 'dead',
				requester_pubkey: 'beef',
				provider_pubkey: 'cafe',
				requester_ssh_pubkey: 'ssh',
				requester_contact: 'contact',
				offering_id: 'offer-42',
				payment_amount_e9s: 500,
				request_memo: 'memo',
				created_at_ns: 0,
				status: 'accepted'
			}
		];

		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: contracts })
		} as Response);

		const result = await getProviderContracts(providerHeaders, providerHex);
		expect(result[0].contract_id).toBe('dead');
		expect(result[0].requester_pubkey).toBe('beef');

		expect(fetch).toHaveBeenCalledWith(
			expect.stringContaining(`/api/v1/providers/${providerHex}/contracts`),
			expect.objectContaining({ method: 'GET', headers: providerHeaders })
		);
	});

	it('throws when HTTP request fails', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: false,
			status: 500,
			statusText: 'Server Error'
		} as Response);

		await expect(getProviderContracts(providerHeaders, providerHex)).rejects.toThrow(
			'Failed to fetch provider contracts: 500 Server Error'
		);
	});

	it('throws when API indicates an error', async () => {
		vi.mocked(fetch).mockResolvedValue({
			ok: true,
			json: async () => ({ success: false, error: 'Unauthorized' })
		} as Response);

		await expect(getProviderContracts(providerHeaders, providerHex)).rejects.toThrow('Unauthorized');
	});
});

describe('getProviderResponseMetrics', () => {
	const sampleMetrics = {
		avgResponseSeconds: 3600,
		avgResponseHours: 1.0,
		slaCompliancePercent: 95.5,
		breachCount30d: 2,
		totalInquiries30d: 40,
		distribution: {
			within1hPct: 60.0,
			within4hPct: 85.0,
			within12hPct: 95.0,
			within24hPct: 98.0,
			within72hPct: 100.0,
			totalResponses: 40
		}
	};

	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('returns response metrics when API succeeds with hex string', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: sampleMetrics })
		});

		const metrics = await getProviderResponseMetrics('abcd1234');

		expect(metrics.avgResponseHours).toBe(1.0);
		expect(metrics.slaCompliancePercent).toBe(95.5);
		expect(metrics.distribution.within1hPct).toBe(60.0);
		expect(globalThis.fetch).toHaveBeenCalledWith(
			expect.stringContaining('/api/v1/providers/abcd1234/response-metrics')
		);
	});

	it('converts Uint8Array to hex string', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: true, data: sampleMetrics })
		});

		const pubkey = new Uint8Array([0xab, 0xcd, 0x12, 0x34]);
		await getProviderResponseMetrics(pubkey);

		expect(globalThis.fetch).toHaveBeenCalledWith(
			expect.stringContaining('/api/v1/providers/abcd1234/response-metrics')
		);
	});

	it('throws when API fails', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: false,
			status: 500,
			statusText: 'Internal Server Error'
		});

		await expect(getProviderResponseMetrics('test')).rejects.toThrow(
			'Failed to fetch response metrics'
		);
	});

	it('throws when API reports failure', async () => {
		globalThis.fetch = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => ({ success: false, error: 'Provider not found' })
		});

		await expect(getProviderResponseMetrics('test')).rejects.toThrow('Provider not found');
	});
});
