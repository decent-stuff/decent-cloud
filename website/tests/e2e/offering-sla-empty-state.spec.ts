import { test, expect } from './fixtures/test-account';
import { sql, nowNs } from './fixtures/seed-helpers';

// Demo provider pubkey (already in provider_registrations) — reuse so we don't
// have to seed a registration row just to satisfy the SLA-target FK.
const DEMO_PROVIDER_PUBKEY_HEX =
	'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';

/**
 * Seed a fresh public offering owned by the demo provider, WITH an SLA target
 * (99.5%) but NO SLI reports. Returns the numeric BIGSERIAL id.
 *
 * This is exactly the state that triggers #435: the SLA card renders (because
 * slaTargetPercent !== undefined) but the SlaBreachTimeline chart shows 30
 * empty gray bars — visually indistinguishable from broken data.
 */
async function seedOfferingWithSlaTarget(): Promise<number> {
	const out = await sql(
		`INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, created_at_ns)
		 VALUES (decode('${DEMO_PROVIDER_PUBKEY_HEX}', 'hex'), 'sla-empty-test-${Date.now()}', 'SLA Empty State Test', 'ICP', 25.0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'Test City', ${nowNs()})
		 RETURNING id`
	);
	const numericId = out
		.split('\n')
		.map((l) => l.trim())
		.find((l) => /^\d+$/.test(l));
	if (!numericId) throw new Error(`could not parse offering id from psql output: ${out}`);

	await sql(
		`INSERT INTO provider_offering_sla_targets (offering_id, provider_pubkey, sla_target_percent, updated_at_ns)
		 VALUES (${numericId}, decode('${DEMO_PROVIDER_PUBKEY_HEX}', 'hex'), 99.5, ${nowNs()})`
	);
	return Number(numericId);
}

async function cleanupOffering(numericId: number): Promise<void> {
	// DELETE on provider_offerings cascades to provider_offering_sla_targets.
	await sql(`DELETE FROM provider_offerings WHERE id = ${numericId}`);
}

test.describe('Offering detail SLA card — empty state (#435)', () => {
	test("shows friendly empty state instead of empty gray bars when provider set an SLA target but has no SLI reports", async ({
		page
	}) => {
		const offeringId = await seedOfferingWithSlaTarget();
		try {
			await page.goto(`/dashboard/marketplace/${offeringId}`);

			// SLA card must be visible — the provider's target IS meaningful info.
			await expect(page.locator('h2', { hasText: 'SLA & Reported Reliability' })).toBeVisible();

			// Promised SLA target renders in the card header.
			await expect(page.getByText('99.50%')).toBeVisible();

			// The empty-state message must be visible.
			await expect(page.getByText('No SLA reports in the last 30 days')).toBeVisible();

			// The SlaBreachTimeline chart (30 gray "No report" bars) must NOT
			// render. The "No report" legend swatch only exists inside the
			// chart component, so its absence proves the chart was replaced.
			await expect(page.getByText('No report', { exact: true })).not.toBeVisible();
		} finally {
			await cleanupOffering(offeringId);
		}
	});
});
