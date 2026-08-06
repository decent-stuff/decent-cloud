// Flow #3 — signup
// Drives the website sign-up flow headlessly (the ONLY account-creation path),
// captures the seed phrase, and asserts a logged-in state. The captured identity
// is stored on ctx.account for downstream provider-onboarding flows.
//
// Cleanup: there is no public "delete account" API, so the test account email is
// noted loudly for manual reconciliation. The downstream provider-onboarding flow
// cleans up the cloud account + offering it creates.

import { signUpViaUi } from '../browser.js';
import { deriveIdentity, pubkeyHex } from '../crypto.js';

const flow = {
	name: 'signup',
	description: 'Website sign-up (seed phrase) → fresh account; assert logged-in state',
	requires: [],
	needsBrowser: true, // runner lazy-launches a shared headless Chromium
	async run(ctx) {
		const { webUrl, accountEmailPrefix } = ctx.config;
		const stamp = `${Date.now()}${Math.floor(Math.random() * 1e4)}`;
		const username = `e2e${stamp}`;
		const email = `${accountEmailPrefix}+${stamp}@e2e.test`;

		ctx.log(`creating account ${username} <${email}> via ${webUrl}`);
		const { seedPhrase, username: uname, email: mail } = await signUpViaUi(ctx.browser, {
			webUrl,
			email,
			username,
		});

		const identity = await deriveIdentity(seedPhrase);
		const pkHex = pubkeyHex(identity);
		ctx.assert(/^[0-9a-f]{64}$/.test(pkHex), `derived pubkey is not 64-char hex: ${pkHex}`);

		ctx.account = { seedPhrase, username: uname, email: mail, identity, pubkeyHex: pkHex };
		ctx.metric('signup.username', uname);
		ctx.metric('signup.pubkey', pkHex);
		ctx.log(`signed up + logged in; pubkey=${pkHex.slice(0, 10)}…`);

		// No public account-delete endpoint — surface the email for reconciliation.
		ctx.note(
			`Test account created: username='${uname}' email='${mail}' pubkey='${pkHex}'. ` +
				`No public delete-account API exists; reconcile manually if this target must stay clean.`,
		);
	},
};

export default flow;
