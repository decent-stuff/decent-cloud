import { test, expect } from './fixtures/test-account';
import type { Page } from '@playwright/test';
import {
	pubkeyHexFromSeed,
	identityFromSeedPhrase,
	signedApiCall,
	verifyAccountEmail,
	deleteContractsForRequester,
	sql,
} from './fixtures/seed-helpers';

/**
 * GATED real-provisioning E2E: rents a REAL Hetzner cx23 cloud-resell VM,
 * waits for it to become `active`, asserts the direct-SSH instance details +
 * the rental-detail SSH command render, then cancels and asserts the VM is
 * cleaned up. This is the only coverage for the real cloud-resell provisioning
 * path (the api-cli `e2e provision` binary exercised it manually in Phase 1;
 * rent-flow.spec.ts only covers a SEEDED resource, never a real VM).
 *
 * ⚠️ THIS SPEC SPENDS REAL MONEY. Renting offering 1628 (db id) creates a real
 * Hetzner cx23 VM billed at ~$6/mo (prorated to cents for the seconds it
 * lives). It is therefore GATED behind `E2E_REAL_PROVISIONING=1` and is a
 * no-op (cleanly skipped) in every default run:
 *   - `npm run test:e2e:fast`           → skipped
 *   - CI without the flag               → skipped
 *   - `E2E_REAL_PROVISIONING=1 npx ...` → runs for real
 *
 * Prerequisites for a real run (operator / next session):
 *   - warm stack up (api :59011, web :59010) with `RATE_LIMIT_ENABLED=false`;
 *   - the API's cloud-provisioning background loop running (it is, by default,
 *     every CLOUD_PROVISIONING_INTERVAL_SECS=10s);
 *   - offering 1628's provider (`hetzner-reseller`, auto_accept_rentals=true)
 *     owning a cloud_account whose stored Hetzner token can create/delete a
 *     cx23 in nbg1.
 *
 * Cleanup discipline: even if an assertion throws, afterAll cancels every
 * non-terminal contract for the test requester so no VM is left running.
 *
 * Wallet math is NOT asserted here — rent-flow.spec.ts already proves the
 * debit/refund amounts exactly. This spec proves the PROVISIONING path
 * (VM create → SSH facts → VM teardown) that rent-flow cannot, because its
 * offering is backed by a seeded (fake) cloud_resource.
 */

// Gate flag. Default-OFF: unset → the whole describe is skipped.
const ENABLED = process.env.E2E_REAL_PROVISIONING === '1';

// db id of the real cloud-resell Hetzner offering (cx23 / nbg1 / ubuntu-24.04).
const REAL_OFFERING_DB_ID = 1628;
// Wallet seed: $10 (1e10 e9s) covers the $6/mo cx23 prorated for the rental
// window with a wide margin (provisioning + a few seconds of active time).
const WALLET_SEED_E9S = 10_000_000_000;
// Provisioning (VM create + boot + cloud-init SSH key) takes ~60s; the polling
// loop fires every 10s. Allow generous headroom before declaring failure.
const PROVISION_TIMEOUT_MS = 180_000;
const TERMINATE_TIMEOUT_MS = 120_000;
const POLL_INTERVAL_MS = 5_000;
// SSH key generated for this spec only (never reused, never on a real host).
const SSH_PUBKEY =
	'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIE2eRealProvSpecKeyData9981 e2e@real-provisioning';
const CONTACT = 'email:e2e-real-provisioning@test.example.com';

test.describe('REAL cloud-resell provisioning (Hetzner) — gated', () => {
	test.describe.configure({ mode: 'serial' });
	// Skip the whole group when the flag is unset. Playwright reports this as
	// "skipped" per test (1 skipped, 0 failed) — a clean no-op.
	test.skip(
		!ENABLED,
		'set E2E_REAL_PROVISIONING=1 to run (rents a REAL Hetzner VM — spends money)',
	);

	let requesterPubkey = '';
	let identity: ReturnType<typeof identityFromSeedPhrase>;
	// The contract created by the rent test, so afterAll can cancel it if the
	// test threw before reaching its own cancel step.
	let createdContractId: string | undefined;

	test.beforeAll(async ({ testAccount }) => {
		requesterPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		identity = identityFromSeedPhrase(testAccount.seedPhrase);
		// Fresh slate: no stale active contract for this pubkey should linger.
		await deleteContractsForRequester(requesterPubkey);
		// Rentals require a verified email (API guard).
		await verifyAccountEmail(requesterPubkey);
		// Seed a prepaid wallet balance so the wallet debit at rent succeeds.
		// ON CONFLICT handles re-runs of the gated spec.
		await sql(`
			INSERT INTO wallet_balances (pubkey, balance_e9s)
			VALUES ('${requesterPubkey}', ${WALLET_SEED_E9S})
			ON CONFLICT (pubkey) DO UPDATE SET balance_e9s = ${WALLET_SEED_E9S}
		`);
	});

	test.afterAll(async () => {
		// SAFETY NET (money): cancel every non-terminal contract for the
		// requester so a real Hetzner VM is never left running because an
		// assertion threw mid-flow. Idempotent + best-effort (logged, never
		// throws — cleanup must not mask the original failure).
		try {
			const raw = await sql(`
				SELECT encode(contract_id, 'hex') FROM contract_sign_requests
				WHERE requester_pubkey = decode('${requesterPubkey}', 'hex')
				  AND status IN ('requested','pending','accepted','provisioning','provisioned','active')
			`);
			const leftover = raw
				.split('\n')
				.map((l) => l.trim())
				.filter((l) => /^[0-9a-f]+$/.test(l));
			for (const id of leftover) {
				try {
					await signedApiCall(identity, 'PUT', `/api/v1/contracts/${id}/cancel`, undefined);
				} catch (err) {
					console.warn(
						`[real-provisioning afterAll] failed to cancel leftover contract ${id}:`,
						err instanceof Error ? err.message : err,
					);
				}
			}
		} catch (err) {
			console.warn(
				'[real-provisioning afterAll] leftover-contract query failed:',
				err instanceof Error ? err.message : err,
			);
		}

		// Teardown wallet + contract rows so sibling specs see a clean state.
		try {
			await deleteContractsForRequester(requesterPubkey);
			await sql(`DELETE FROM wallet_ledger WHERE pubkey = '${requesterPubkey}'`);
			await sql(`DELETE FROM wallet_balances WHERE pubkey = '${requesterPubkey}'`);
		} catch (err) {
			console.warn(
				'[real-provisioning afterAll] wallet/contract teardown failed:',
				err instanceof Error ? err.message : err,
			);
		}
	});

	/**
	 * Poll the contract status from the DB until it reaches `target` or a
	 * terminal non-target state. Returns the final status string. DB-direct
	 * (no per-poll signed request) so polling is cheap and bounded.
	 */
	async function pollContractStatus(
		contractId: string,
		target: string,
		timeoutMs: number,
	): Promise<string> {
		const deadline = Date.now() + timeoutMs;
		let last = '';
		const terminal = new Set(['cancelled', 'rejected', 'failed', 'expired']);
		while (Date.now() < deadline) {
			const out = await sql(
				`SELECT status FROM contract_sign_requests WHERE contract_id = decode('${contractId}', 'hex')`,
			);
			last = out.trim();
			if (last === target) return last;
			// A terminal state that is NOT the target means provisioning/cancel
			// failed — stop early with the real status so the assertion message
			// is informative.
			if (terminal.has(last)) return last;
			await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS));
		}
		throw new Error(
			`contract ${contractId} never reached status '${target}' within ${timeoutMs}ms (last='${last}')`,
		);
	}

	/** Read + parse the provisioning_instance_details JSON for a contract. */
	async function readInstanceDetails(contractId: string): Promise<Record<string, unknown>> {
		const raw = await sql(
			`SELECT provisioning_instance_details FROM contract_sign_requests WHERE contract_id = decode('${contractId}', 'hex')`,
		);
		if (!raw) throw new Error(`no provisioning_instance_details for contract ${contractId}`);
		return JSON.parse(raw);
	}

	test('rent → provision → SSH details → cancel (real Hetzner VM)', async ({ page }) => {
		// 1. Rent the real offering via a signed POST /api/v1/contracts. Direct
		//    API call (not the marketplace dialog) keeps this spec focused on
		//    the PROVISIONING path and avoids depending on the offering's
		//    marketplace visibility — rent-flow.spec.ts already covers the
		//    dialog. payment_method=wallet debits the seeded balance.
		const rentResp = await signedApiCall(identity, 'POST', '/api/v1/contracts', {
			offering_db_id: REAL_OFFERING_DB_ID,
			ssh_pubkey: SSH_PUBKEY,
			payment_method: 'wallet',
			contact_method: CONTACT,
		});
		expect(rentResp.ok, `rent POST must succeed: ${await rentResp.text().catch(() => '<no body>')}`).toBe(true);
		const rentJson = await rentResp.json();
		const contractId: string = rentJson?.data?.contractId;
		expect(contractId, `rent response must carry data.contractId: ${JSON.stringify(rentJson)}`).toMatch(
			/^[0-9a-f]+$/,
		);
		createdContractId = contractId;

		// 2. Wait for provisioning to finish. The provider auto-accepts (its
		//    profile has auto_accept_rentals=true) → the API's cloud loop
		//    creates the cx23, injects the SSH key, and flips the contract to
		//    `active`. ~60s in practice; 180s budget.
		const activeStatus = await pollContractStatus(contractId, 'active', PROVISION_TIMEOUT_MS);
		expect(
			activeStatus,
			`real Hetzner VM must reach 'active' (got '${activeStatus}' — provisioning likely failed; check api logs + the offering's cloud_account Hetzner token)`,
		).toBe('active');

		// 3. Assert the direct-SSH instance details. Cloud-resell (Model B) VMs
		//    SSH directly to root@<public_ip>:22 — NO gateway. The JSON MUST
		//    carry connection_type=direct_ssh + public_ip + ssh_port=22 and
		//    MUST NOT carry any gateway_* field (those would mislead consumers
		//    toward a gateway path that does not exist for cloud-resell).
		const details = await readInstanceDetails(contractId);
		expect(details.connection_type).toBe('direct_ssh');
		expect(details.public_ip, 'public_ip must be present').toBeTruthy();
		expect(String(details.public_ip)).toMatch(/^\d{1,3}(\.\d{1,3}){3}$/);
		expect(details.ssh_port).toBe(22);
		expect(details.gateway_slug, 'gateway_slug must be absent for direct_ssh').toBeUndefined();
		expect(details.gateway_subdomain, 'gateway_subdomain must be absent for direct_ssh').toBeUndefined();
		expect(details.gateway_ssh_port, 'gateway_ssh_port must be absent for direct_ssh').toBeUndefined();
		const publicIp = String(details.public_ip);

		// 4. The rental detail page must render the SSH command for the
		//    provisioned IP. The rendered username is OS-derived
		//    (sshUsername(): "ubuntu" for ubuntu images, "root" otherwise) —
		//    the backend's direct-SSH intent documents root@<ip>, so this
		//    assertion intentionally matches the IP + `ssh <user>@` prefix and
		//    does NOT hardcode the username. If the rendered user disagrees
		//    with the VM's actual login user, that is a separate UI/backend
		//    mismatch this spec surfaces (not a provisioning failure).
		await page.goto(`/dashboard/rentals/${contractId}`);
		// Wait for the detail page to mount the connection block (the IP
		// appears in the SSH command code + the IP-address block).
		await expect(page.locator('body')).toContainText(publicIp, { timeout: 15_000 });
		const sshCode = page.locator('code', { hasText: new RegExp(`ssh\\s+\\S+@${publicIp.replace(/\./g, '\\.')}`) });
		await expect(sshCode.first()).toBeVisible({ timeout: 15_000 });

		// 5. Cancel the rental. `active` is cancellable; the API refunds the
		//    wallet (prorated) and marks the linked cloud_resource for deletion.
		const cancelResp = await signedApiCall(
			identity,
			'PUT',
			`/api/v1/contracts/${contractId}/cancel`,
			undefined,
		);
		expect(
			cancelResp.ok,
			`cancel PUT must succeed: ${await cancelResp.text().catch(() => '<no body>')}`,
		).toBe(true);

		// 6a. Contract reaches 'cancelled'.
		const finalStatus = await pollContractStatus(contractId, 'cancelled', 60_000);
		expect(finalStatus).toBe('cancelled');

		// 6b. The linked cloud_resource is cleaned up: the cancel marks it
		//     'deleting', then the termination loop deletes the Hetzner VM and
		//     flips the row to 'deleted' (terminated_at set). Poll for the
		//     terminal 'deleted' state so the spec only passes once the billable
		//     VM is actually gone.
		const terminateDeadline = Date.now() + TERMINATE_TIMEOUT_MS;
		let resourceStatus = '';
		while (Date.now() < terminateDeadline) {
			const out = await sql(
				`SELECT status FROM cloud_resources WHERE contract_id = decode('${contractId}', 'hex') LIMIT 1`,
			);
			resourceStatus = out.trim();
			if (resourceStatus === 'deleted') break;
			await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS));
		}
		expect(
			resourceStatus,
			`cloud_resource for cancelled contract must reach 'deleted' (VM terminated at Hetzner); last='${resourceStatus}'`,
		).toBe('deleted');

		// Mark cleaned up so afterAll doesn't redundantly retry cancel.
		createdContractId = undefined;
	});
});
