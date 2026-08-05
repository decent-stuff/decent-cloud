// Flow #5 (OPTIONAL, GATED, COSTS MONEY) — rent-provision-cancel
// Only runs when config.includeProvision === true. Creates a real rental
// contract on the provider-onboard offering, waits for the VM to provision,
// asserts SSH :22 reachability, then cancels + asserts cleanup.
//
// This flow SPENDS REAL MONEY (provisions a real Hetzner VM, forced to cx22 per
// MINIMIZE-CLOUD-SPENDING). It requires: a Hetzner token, a target whose API
// auto-provisions Path-A offerings, working payment/self-rental, and network
// egress to the provisioned VM. Cleanup is bulletproof (try/finally cancel).
//
// It is OFF by default; enable with DC_E2E_INCLUDE_PROVISION=1 / --include-provision.

import { failDetail, excerpt } from '../http.js';

const PROVISION_TIMEOUT_MS = 8 * 60_000;
const POLL_INTERVAL_MS = 10_000; // background-task polling: never faster than 10s

// A throwaway ed25519 pubkey is rejected by SSH key format validation; use a
// syntactically-valid placeholder. Provisioned VMs are cancelled immediately.
const DUMMY_SSH_PUBKEY = 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIE2eharnessplaceholderkeye2etest e2e-harness';

const flow = {
	name: 'rent-provision-cancel',
	description: '[GATED] Real rent→provision→SSH:22→cancel on a cx22 offering (spends money)',
	requires: ['provider-onboard-path-a'],
	async run(ctx) {
		if (!ctx.config.includeProvision) {
			ctx.note('rent-provision-cancel skipped: DC_E2E_INCLUDE_PROVISION is not enabled.');
			return;
		}
		const { identity, pubkeyHex } = ctx.account;
		const offeringId = ctx.offering?.id;
		ctx.assert(typeof offeringId === 'number', 'no offering available — provider-onboard-path-a must run first');

		let contractId = null;
		try {
			// 1. Create the rental contract (self-rental: requester == provider).
			ctx.log(`creating rental contract on offering ${offeringId} (cx22)…`);
			const create = await ctx.signed(identity, 'POST', '/api/v1/contracts', {
				offering_db_id: offeringId,
				ssh_pubkey: DUMMY_SSH_PUBKEY,
				payment_method: 'stripe',
				duration_hours: 1,
			});
			ctx.assert(create.json?.success === true, failDetail('POST /contracts must succeed', create));
			contractId = create.json?.data?.contract_id;
			ctx.assert(typeof contractId === 'string' && contractId.length > 0, failDetail('contract create missing contract_id', create));
			ctx.metric('contract.id', contractId);
			ctx.log(`contract ${contractId} created`);

			// 2. Wait for provisioning to reach an active/provisioned terminal state.
			ctx.log('waiting for VM provisioning (this provisions a REAL cx22)…');
			const terminal = await waitForProvisioned(ctx, identity, contractId);

			// 3. Verify SSH :22 is reachable on the provisioned host.
			const host = terminal.publicIp ?? terminal.gatewaySubdomain;
			ctx.assert(host, 'provisioned contract exposes no reachable host (no publicIp/gatewaySubdomain)');
			const reachable = await isPortReachable(host, terminal.gatewaySshPort ?? 22);
			ctx.assert(reachable, `SSH port ${terminal.gatewaySshPort ?? 22} on ${host} not reachable after provisioning`);
			ctx.metric('ssh.host', host);
			ctx.log(`VM reachable at ${host}:${terminal.gatewaySshPort ?? 22}`);
		} finally {
			// 4. Cancel — always. Then assert no leftover.
			if (contractId) {
				const cancel = await ctx.signed(identity, 'POST', `/api/v1/contracts/${contractId}/cancel`, {
					reason: 'e2e-harness rent-provision-cancel cleanup',
				});
				if (!cancel.json?.success) {
					ctx.note(`cleanup: cancel of contract ${contractId} reported non-success: ${excerpt(cancel.json ?? cancel.text)}`);
				} else {
					ctx.log(`contract ${contractId} cancelled`);
				}
				// Best-effort: confirm cancelled terminal state.
				const final = await ctx.signed(identity, 'GET', `/api/v1/contracts/${contractId}`);
				const status = final.json?.data?.status;
				if (status && status !== 'cancelled' && status !== 'terminated') {
					ctx.note(`cleanup: contract ${contractId} ended in status '${status}' (expected cancelled/terminated)`);
				}
			}
		}
	},
};

/** Poll the contract until it is provisioned/active or fails/times out. */
async function waitForProvisioned(ctx, identity, contractId) {
	const start = Date.now();
	for (;;) {
		if (Date.now() - start > PROVISION_TIMEOUT_MS) {
			ctx.assert(false, `contract ${contractId} did not provision within ${PROVISION_TIMEOUT_MS / 1000}s`);
		}
		const r = await ctx.signed(identity, 'GET', `/api/v1/contracts/${contractId}`);
		const data = r.json?.data;
		const status = data?.status;
		ctx.log(`contract status: ${status} (${Math.round((Date.now() - start) / 1000)}s)`);
		if (status === 'active' || status === 'provisioned') {
			return {
				status,
				publicIp: data.publicIp ?? data.public_ip ?? null,
				gatewaySubdomain: data.gatewaySubdomain ?? data.gateway_subdomain ?? null,
				gatewaySshPort: data.gatewaySshPort ?? data.gateway_ssh_port ?? null,
			};
		}
		if (status === 'failed') {
			ctx.assert(false, failDetail('contract provisioning failed (status=failed)', r));
		}
		await sleep(POLL_INTERVAL_MS);
	}
}

function sleep(ms) {
	return new Promise((r) => setTimeout(r, ms));
}

/** TCP-reachability probe for the SSH port (best-effort; no SSH banner parse). */
async function isPortReachable(host, port) {
	try {
		const { default: net } = await import('node:net');
		return await new Promise((resolve) => {
			const sock = new net.Socket();
			const t = setTimeout(() => done(false), 10_000);
			const done = (ok) => {
				clearTimeout(t);
				sock.destroy();
				resolve(ok);
			};
			sock.once('connect', () => done(true));
			sock.once('error', () => done(false));
			sock.connect(port, host);
		});
	} catch {
		return false;
	}
}

export default flow;
