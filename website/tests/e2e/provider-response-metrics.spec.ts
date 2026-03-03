import { test, expect } from '@playwright/test';

const API_BASE_URL = process.env.VITE_DECENT_CLOUD_API_URL || 'http://localhost:59011';

test.describe('Provider Contract Request Response Metrics', () => {
	test('@smoke GET /providers/:pubkey/response-metrics returns contract request SLA metrics', async ({
		request
	}) => {
		const validPubkey = '0'.repeat(64);
		const response = await request.get(
			`${API_BASE_URL}/api/v1/providers/${validPubkey}/response-metrics`
		);

		expect(response.status()).toBe(200);

		const data = await response.json();
		expect(data.success).toBe(true);
		expect(data.data).toHaveProperty('avgResponseSeconds');
		expect(data.data).toHaveProperty('avgResponseHours');
		expect(data.data).toHaveProperty('slaCompliancePercent');
		expect(data.data).toHaveProperty('breachCount30d');
		expect(data.data).toHaveProperty('totalInquiries30d');
		expect(data.data).toHaveProperty('distribution');
		expect(data.data.distribution).toHaveProperty('within1hPct');
		expect(data.data.distribution).toHaveProperty('within4hPct');
		expect(data.data.distribution).toHaveProperty('within12hPct');
		expect(data.data.distribution).toHaveProperty('within24hPct');
		expect(data.data.distribution).toHaveProperty('within72hPct');
		expect(data.data.distribution).toHaveProperty('totalResponses');
	});

	test('@smoke GET /providers/:pubkey/response-metrics returns error for invalid pubkey in contract request metrics', async ({
		request
	}) => {
		const response = await request.get(
			`${API_BASE_URL}/api/v1/providers/invalid-pubkey/response-metrics`
		);

		expect(response.status()).toBe(200);
		const data = await response.json();
		expect(data.success).toBe(false);
		expect(data.error).toContain('Invalid pubkey format');
	});
});
