// Flow #4 — provider-onboard-path-a
// The provider-onboard (Path A) flow: resell a managed cloud, gateway-free. As
// the signed-up user, via SIGNED API calls:
//   1. register a provider_profile (PUT /providers/:pubkey/onboarding)
//   2. add the Hetzner cloud account (POST /cloud-accounts — token validated LIVE)
//   3. assert the catalog live-fetches (GET /cloud-accounts/:id/catalog)
//   4. create ONE offering from the catalog (cheapest server type)
//   5. assert it appears in the provider's offerings
// Cleanup: delete the offering + cloud account (best-effort; failures are noted).
//
// Requires the signup flow to have run (ctx.account).

import { assertEquals } from '../assert.js';
import { failDetail, excerpt } from '../http.js';
import { signRequest } from '../crypto.js';

const DISPLAY_NAME = 'E2E Harness Provider (automated)';

const flow = {
	name: 'provider-onboard-path-a',
	description: 'Provider-onboard (Path A) flow: provider_profile → Hetzner cloud account → catalog → offering',
	requires: ['signup'],
	async run(ctx) {
		const { apiUrl, hetznerToken } = ctx.config;
		const { identity, pubkeyHex } = ctx.account;
		let cloudAccountId = null;
		let offeringId = null;

		try {
			// 1. Register provider_profile.
			ctx.log('registering provider_profile…');
			const onboard = await ctx.signed(identity, 'PUT', `/api/v1/providers/${pubkeyHex}/onboarding`, {
				support_email: `${ctx.account.username}@e2e.test`,
				support_hours: '24/7',
				support_channels: 'Email',
				regions: 'Europe',
				payment_methods: 'Stripe',
				refund_policy: 'Test provider — automated e2e harness.',
				sla_guarantee: 'Best-effort (automated test offering).',
			});
			ctx.assert(onboard.json?.success === true, failDetail('provider onboarding PUT must succeed', onboard));

			// 2. Add the Hetzner cloud account (token is validated LIVE by the API).
			ctx.log('adding Hetzner cloud account (live token validation)…');
			const add = await ctx.signed(identity, 'POST', '/api/v1/cloud-accounts', {
				backend_type: 'hetzner',
				name: `e2e-hetzner-${Date.now()}`,
				credentials: hetznerToken,
			});
			ctx.assert(add.json?.success === true, failDetail('POST /cloud-accounts must succeed', add));
			cloudAccountId = add.json?.data?.id;
			ctx.assert(typeof cloudAccountId === 'string', failDetail('cloud account response missing data.id', add));
			const isValid = add.json?.data?.is_valid;
			ctx.assertEquals(isValid, true, 'cloud account must be is_valid (live token validation)');
			ctx.metric('cloud_account.id', cloudAccountId);
			ctx.log(`cloud account ${cloudAccountId} added + validated`);

			// 3. Catalog must live-fetch server types / locations / images.
			ctx.log('fetching Hetzner catalog via the cloud account…');
			const catalog = await ctx.signed(identity, 'GET', `/api/v1/cloud-accounts/${cloudAccountId}/catalog`);
			ctx.assert(catalog.json?.success === true, failDetail('GET /cloud-accounts/:id/catalog must succeed', catalog));
			const cat = catalog.json?.data;
			ctx.assert(
				Array.isArray(cat?.server_types) && cat.server_types.length > 0,
				failDetail('catalog must return a non-empty server_types array', catalog),
			);
			ctx.assert(Array.isArray(cat?.locations) && cat.locations.length > 0, 'catalog locations missing');
			ctx.assert(Array.isArray(cat?.images) && cat.images.length > 0, 'catalog images missing');
			ctx.metric('catalog.server_types', cat.server_types.length);
			ctx.metric('catalog.locations', cat.locations.length);
			ctx.metric('catalog.images', cat.images.length);

		// 4. Pick the cheapest server type (prefer cx23 per MINIMIZE-CLOUD-SPENDING;
		//    cx22 was retired by Hetzner, cx23 is the cheapest shared-CPU type now),
		//    the first location, and an Ubuntu image.
			const cheapest = pickCheapestServerType(cat.server_types);
			const location = pickLocation(cat.locations);
			const image = pickImage(cat.images);
			ctx.log(`building offering from ${cheapest.name} @ ${location.name} (${image.name})`);

			// 5. Create the offering (provisioner_type=hetzner; provisioner_config
			//    validated LIVE against Hetzner).
			const offering = buildOffering(ctx, cheapest, location, image);
			const create = await ctx.signed(identity, 'POST', `/api/v1/providers/${pubkeyHex}/offerings`, offering);
			ctx.assert(create.json?.success === true, failDetail('POST offering must succeed', create));
			offeringId = create.json?.data;
			ctx.assert(typeof offeringId === 'number', failDetail('offering create response must return a numeric id', create));
			ctx.metric('offering.id', offeringId);
			ctx.log(`offering ${offeringId} created (provisioner=hetzner)`);

			// 6. Assert it appears in the provider's offerings list.
			const list = await ctx.signed(identity, 'GET', `/api/v1/providers/${pubkeyHex}/offerings`);
			ctx.assert(Array.isArray(list.json?.data), failDetail('GET provider offerings must return a data array', list));
			const found = list.json.data.some((o) => o.id === offeringId);
			ctx.assert(found, `offering ${offeringId} not present in provider offerings list`);

			ctx.offering = { id: offeringId, name: offering.offer_name };
			ctx.log(`provider onboard complete; offering ${offeringId} visible`);
		} finally {
			// Best-effort cleanup: offering first, then cloud account. Never swallow
			// silently — surface cleanup failures as findings.
			if (offeringId !== null) {
				const r = await ctx.signed(identity, 'DELETE', `/api/v1/providers/${pubkeyHex}/offerings/${offeringId}`);
				if (!r.json?.success) ctx.note(`cleanup: failed to delete offering ${offeringId}: ${excerpt(r.json ?? r.text)}`);
			}
			if (cloudAccountId !== null) {
				const r = await ctx.signed(identity, 'DELETE', `/api/v1/cloud-accounts/${cloudAccountId}`);
				if (!r.json?.success) ctx.note(`cleanup: failed to delete cloud account ${cloudAccountId}: ${excerpt(r.json ?? r.text)}`);
			}
		}
	},
};

/** Prefer cx23 (cheapest Hetzner shared-CPU; cx22 was retired); else the lowest monthly price; else the first. */
function pickCheapestServerType(serverTypes) {
	const cx23 = serverTypes.find((s) => s.name === 'cx23');
	if (cx23) return cx23;
	const priced = serverTypes.filter((s) => typeof s.priceMonthly === 'number' && s.priceMonthly >= 0);
	if (priced.length) return priced.sort((a, b) => a.priceMonthly - b.priceMonthly)[0];
	return serverTypes[0];
}

function pickLocation(locations) {
	// Prefer a default Hetzner location for determinism, else the first.
	return locations.find((l) => l.name === 'nbg1') ?? locations[0];
}

function pickImage(images) {
	// Prefer ubuntu-24.04, else any ubuntu, else the first.
	return (
		images.find((i) => i.name === 'ubuntu-24.04') ??
		images.find((i) => /ubuntu/i.test(i.name)) ??
		images[0]
	);
}

/** Build a minimal-but-complete Offering payload that passes server validation. */
function buildOffering(ctx, serverType, location, image) {
	const stamp = `${Date.now()}`.slice(-8);
	return {
		// Server sets id + pubkey; everything below mirrors the website wizard.
		offering_id: `e2e-cx23-${stamp}`,
		offer_name: `[e2e] ${serverType.name} @ ${location.name} (automated test)`,
		description: 'Automated e2e-harness offering — safe to delete.',
		currency: 'eur',
		monthly_price: Math.max(5, Math.ceil((serverType.priceMonthly ?? 4.5) * 1.5)),
		setup_fee: 0,
		visibility: 'public',
		product_type: 'vps',
		virtualization_type: 'kvm',
		billing_interval: 'monthly',
		billing_unit: 'month',
		is_subscription: true,
		subscription_interval_days: 30,
		stock_status: 'in_stock',
		processor_cores: serverType.cores,
		memory_amount: `${serverType.memoryGb} GB`,
		total_ssd_capacity: `${serverType.diskGb} GB`,
		unmetered_bandwidth: false,
		datacenter_country: location.country ?? '',
		datacenter_city: location.city ?? location.name,
		operating_systems: image.name,
		is_draft: false,
		is_example: false,
		provisioner_type: 'hetzner',
		provisioner_config: JSON.stringify({
			server_type: serverType.name,
			location: location.name,
			image: image.name,
		}),
	};
}

export default flow;
