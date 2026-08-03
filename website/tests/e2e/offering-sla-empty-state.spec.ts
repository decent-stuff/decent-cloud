import { test, expect } from './fixtures/test-account';
import {
	sql,
	nowNs,
	randomHex,
	seedOffering,
	seedProviderRegistration,
} from './fixtures/seed-helpers';

/**
 * Seed a fresh public offering under a THROWAWAY provider pubkey, WITH an SLA
 * target (99.5%) but NO SLI reports. Returns the numeric BIGSERIAL id.
 *
 * This is exactly the state that triggers #435: the SLA card renders (because
 * slaTargetPercent !== undefined) but the SlaBreachTimeline chart shows 30
 * empty gray bars — visually indistinguishable from broken data.
 *
 * The drop-demos pivot (migration 053) deleted the example provider's
 * `provider_registrations` row, which `provider_offering_sla_targets` FKs on
 * `provider_pubkey`. So this spec now seeds its OWN registration row under a
 * fresh random pubkey instead of reusing the (gone) demo provider — fully
 * self-contained, no collision with other specs.
 */
async function seedOfferingWithSlaTarget(): Promise<{ id: number; providerPubkeyHex: string }> {
	const providerPubkeyHex = randomHex(32);
	await seedProviderRegistration(providerPubkeyHex);

	const numericId = await seedOffering(providerPubkeyHex, {
		name: 'SLA Empty State Test',
	});

	await sql(
		`INSERT INTO provider_offering_sla_targets (offering_id, provider_pubkey, sla_target_percent, updated_at_ns)
		 VALUES (${numericId}, decode('${providerPubkeyHex}', 'hex'), 99.5, ${nowNs()})`
	);
	return { id: Number(numericId), providerPubkeyHex };
}

async function cleanupOffering(id: number, providerPubkeyHex: string): Promise<void> {
	// DELETE on provider_offerings cascades to provider_offering_sla_targets.
	await sql(`DELETE FROM provider_offerings WHERE id = ${id}`);
	await sql(`DELETE FROM provider_registrations WHERE pubkey = decode('${providerPubkeyHex}', 'hex')`);
}

test.describe('Offering detail SLA card — empty state (#435)', () => {
	test("shows friendly empty state instead of empty gray bars when provider set an SLA target but has no SLI reports", async ({
		page
	}) => {
		const { id: offeringId, providerPubkeyHex } = await seedOfferingWithSlaTarget();
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
			await cleanupOffering(offeringId, providerPubkeyHex);
		}
	});
});
