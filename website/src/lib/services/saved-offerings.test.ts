import { describe, it, expect, vi, afterEach } from 'vitest';
import { toggleSavedId } from './saved-offerings';
import { getSavedOfferingIds, saveOffering, unsaveOffering } from './api';
import type { SignedRequestHeaders } from '$lib/types/generated/SignedRequestHeaders';

vi.mock('./api', () => ({
	getSavedOfferingIds: vi.fn(),
	saveOffering: vi.fn(),
	unsaveOffering: vi.fn()
}));

const headers: SignedRequestHeaders = {
	'X-Public-Key': 'aabbcc',
	'X-Signature': '00112233',
	'X-Timestamp': '1000000',
	'X-Nonce': 'test-nonce',
	'Content-Type': 'application/json'
};

describe('toggleSavedId', () => {
	it('adds an ID that is not in the set', () => {
		const result = toggleSavedId(new Set([1, 2]), 3);
		expect(result.has(3)).toBe(true);
		expect(result.size).toBe(3);
	});

	it('removes an ID that is already in the set', () => {
		const result = toggleSavedId(new Set([1, 2, 3]), 2);
		expect(result.has(2)).toBe(false);
		expect(result.size).toBe(2);
	});

	it('returns a new Set (does not mutate the original)', () => {
		const original = new Set([1, 2]);
		const result = toggleSavedId(original, 3);
		expect(original.size).toBe(2);
		expect(result).not.toBe(original);
	});

	it('handles empty set by adding the ID', () => {
		const result = toggleSavedId(new Set(), 5);
		expect(result.size).toBe(1);
		expect(result.has(5)).toBe(true);
	});
});

describe('getSavedOfferingIds API response handling', () => {
	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('returns array of IDs from successful API response', async () => {
		vi.mocked(getSavedOfferingIds).mockResolvedValue([1, 2, 3]);

		const result = await getSavedOfferingIds(headers, 'aabbcc');
		expect(result).toEqual([1, 2, 3]);
	});

	it('returns empty array when no saved offerings', async () => {
		vi.mocked(getSavedOfferingIds).mockResolvedValue([]);

		const result = await getSavedOfferingIds(headers, 'aabbcc');
		expect(result).toEqual([]);
	});

	it('propagates error on API failure', async () => {
		vi.mocked(getSavedOfferingIds).mockRejectedValue(new Error('Failed to fetch saved offering IDs: 401 Unauthorized'));

		await expect(getSavedOfferingIds(headers, 'aabbcc')).rejects.toThrow('401');
	});
});

describe('saveOffering / unsaveOffering', () => {
	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('calls saveOffering without error on success', async () => {
		vi.mocked(saveOffering).mockResolvedValue(undefined);
		await expect(saveOffering(headers, 'aabbcc', 42)).resolves.toBeUndefined();
	});

	it('propagates error when saveOffering fails', async () => {
		vi.mocked(saveOffering).mockRejectedValue(new Error('Failed to save offering: 404 Not Found'));
		await expect(saveOffering(headers, 'aabbcc', 99)).rejects.toThrow('404');
	});

	it('calls unsaveOffering without error on success', async () => {
		vi.mocked(unsaveOffering).mockResolvedValue(undefined);
		await expect(unsaveOffering(headers, 'aabbcc', 42)).resolves.toBeUndefined();
	});

	it('propagates error when unsaveOffering fails', async () => {
		vi.mocked(unsaveOffering).mockRejectedValue(new Error('Failed to unsave offering: 404 Not Found'));
		await expect(unsaveOffering(headers, 'aabbcc', 99)).rejects.toThrow('404');
	});
});
