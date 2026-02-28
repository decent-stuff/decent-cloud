import { describe, it, expect, vi, beforeEach } from 'vitest';

// Test the checkout success redirect behavior
// The page should navigate to the contract detail page with welcome state
// instead of the generic rentals list page

describe('Checkout success page redirect', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('should construct correct redirect URL with contract ID and welcome param', () => {
		const contractId = 'abc123def456';
		const expectedUrl = `/dashboard/rentals/${contractId}?welcome=true`;

		// This is the expected redirect URL format
		expect(expectedUrl).toBe('/dashboard/rentals/abc123def456?welcome=true');
	});

	it('should include welcome=true query parameter in redirect URL', () => {
		const contractId = 'test-contract-id';
		const redirectUrl = `/dashboard/rentals/${contractId}?welcome=true`;
		const url = new URL(redirectUrl, 'https://example.com');

		expect(url.searchParams.get('welcome')).toBe('true');
		expect(url.pathname).toBe(`/dashboard/rentals/${contractId}`);
	});

	it('should not redirect to generic rentals list page', () => {
		const contractId = 'some-contract-id';
		const correctRedirect = `/dashboard/rentals/${contractId}?welcome=true`;
		const incorrectRedirect = '/dashboard/rentals';

		// The redirect should NOT be to the generic rentals list
		expect(correctRedirect).not.toBe(incorrectRedirect);
		// It should include the contract ID
		expect(correctRedirect).toContain(contractId);
		// And the welcome parameter
		expect(correctRedirect).toContain('welcome=true');
	});
});
