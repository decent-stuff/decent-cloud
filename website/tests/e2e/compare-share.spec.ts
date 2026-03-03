import { test, expect } from '@playwright/test';

const offeringFixtures = {
	101: {
		id: 101,
		offer_name: 'Starter KVM',
		owner_username: 'provider101',
		pubkey: 'a'.repeat(64),
		monthly_price: 9,
		currency: 'ICP',
		billing_interval: 'monthly',
		setup_fee: 0,
		min_contract_hours: 24,
		max_contract_hours: 720,
		processor_cores: 4,
		memory_amount: '8 GB',
		virtualization_type: 'KVM',
		total_ssd_capacity: '80 GB',
		total_hdd_capacity: '0 GB',
		uplink_speed: '1 Gbps',
		unmetered_bandwidth: false,
		traffic: 10,
		datacenter_city: 'Berlin',
		datacenter_country: 'Germany',
		trust_score: 82,
		reliability_score: 97,
		offering_source: 'native',
		is_example: false,
	},
	202: {
		id: 202,
		offer_name: 'Scale KVM',
		owner_username: 'provider202',
		pubkey: 'b'.repeat(64),
		monthly_price: 14,
		currency: 'ICP',
		billing_interval: 'monthly',
		setup_fee: 0,
		min_contract_hours: 24,
		max_contract_hours: 720,
		processor_cores: 8,
		memory_amount: '16 GB',
		virtualization_type: 'KVM',
		total_ssd_capacity: '120 GB',
		total_hdd_capacity: '0 GB',
		uplink_speed: '1 Gbps',
		unmetered_bandwidth: true,
		traffic: 0,
		datacenter_city: 'Prague',
		datacenter_country: 'Czechia',
		trust_score: 88,
		reliability_score: 98,
		offering_source: 'native',
		is_example: false,
	},
} as const;

test.describe('Marketplace compare sharing', () => {
	test('@smoke copies canonical comparison URL and shows success feedback', async ({ page }) => {
		await page.route('**/api/v1/prices/icp', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ priceUsd: 12.34 }),
			});
		});

		await page.route('**/api/v1/offerings/*', async (route) => {
			const url = new URL(route.request().url());
			const id = Number(url.pathname.split('/').pop());
			const offering = offeringFixtures[id as 101 | 202];

			if (!offering) {
				await route.fulfill({ status: 404, contentType: 'application/json', body: JSON.stringify({ success: false, error: 'not found' }) });
				return;
			}

			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ success: true, data: offering }),
			});
		});

		await page.goto('/dashboard/marketplace/compare?ids=202,101,202');
		await expect(page).toHaveURL('/dashboard/marketplace/compare?ids=202,101');

		await page.getByRole('button', { name: 'Share comparison' }).click();
		await expect(page.getByText('Comparison link copied to clipboard')).toBeVisible();

		const clipboardText = await page.evaluate(async () => navigator.clipboard.readText());
		expect(clipboardText).toBe(`${new URL(page.url()).origin}/dashboard/marketplace/compare?ids=202,101`);
	});
});
