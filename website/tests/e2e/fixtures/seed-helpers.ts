/**
 * DB seed helpers for E2E tests.
 *
 * These helpers insert test data directly into PostgreSQL, bypassing the API
 * signature/auth layer. They're used by specs that need to assert how a page
 * renders populated state when seeding via the real API would require complex
 * signed requests (e.g. contracts require an offering + provider signature).
 *
 * Connection parameters come from DATABASE_URL (defaults to the dev container
 * sidecar at hostname `postgres`). The same env handled by test-admin-account.
 */
import { execFile } from 'child_process';
import { promisify } from 'util';
import { Ed25519KeyIdentity } from '@dfinity/identity';
import { generateMnemonic, mnemonicToSeedSync, validateMnemonic } from 'bip39';
import { hmac } from '@noble/hashes/hmac';
import { sha512 } from '@noble/hashes/sha512';
import { ed25519ph } from '@noble/curves/ed25519';
import { API_BASE_URL } from './api-base';

const execFileAsync = promisify(execFile);

const DATABASE_URL = process.env.DATABASE_URL || 'postgres://test:test@postgres:5432/test';

/**
 * Bounded timeout for every psql invocation. Without it, a `psql` process that
 * blocks on a DB row lock — e.g. the worker-scoped `testAccount` teardown's
 * `DELETE FROM accounts` racing an in-flight API transaction that took a
 * FOR-KEY-SHARE lock on the parent accounts row — waits forever. Worker-scoped
 * fixture teardown runs OUTSIDE the per-test timeout, so that single stuck
 * `psql` hung the whole suite (A2: smoke stalled under 2+ workers; serial mode
 * always passed because there was no parallel API traffic to create the race).
 * 15s is generous for any legitimate seed op while still failing fast on a
 * stuck query; override per-call via `sql({ timeoutMs })`.
 */
const DEFAULT_PSQL_TIMEOUT_MS = 15_000;

function psqlArgs(): { args: string[]; env: NodeJS.ProcessEnv } {
	const url = new URL(DATABASE_URL);
	const args = [
		'--host', url.hostname || 'postgres',
		'--port', url.port || '5432',
		'--username', url.username || 'test',
		'--dbname', url.pathname.replace(/^\//, '') || 'test',
		'--no-psqlrc',
		'--tuples-only',
		'--no-align',
	];
	const env = { ...process.env, PGPASSWORD: url.password || 'test' };
	return { args, env };
}

/**
 * Run one `psql --command` and return trimmed stdout, bounded by `timeoutMs`.
 * Centralized so EVERY seed/cleanup query inherits the timeout (A2 fix): a
 * stuck query is SIGTERM'd at the timeout instead of hanging the worker.
 * `sql()` exposes `timeoutMs` for callers/tests; the RETURNING-id helpers use
 * the default.
 */
async function psqlExec(
	command: string,
	timeoutMs: number = DEFAULT_PSQL_TIMEOUT_MS,
): Promise<string> {
	const { args, env } = psqlArgs();
	const { stdout } = await execFileAsync('psql', [...args, '--command', command], {
		env,
		timeout: timeoutMs,
	});
	return stdout.trim();
}

/**
 * Run a SQL command via psql; returns trimmed stdout.
 * Throws on non-zero exit. Use $1/$2/... placeholders in `sql` and pass
 * values in `params` — they're bound safely via psql --v variables.
 *
 * For tests we only need INSERTs/UPDATEs with bytea literals; building them
 * from hex with `decode(..., 'hex')` is safe against SQL injection because
 * callers control all inputs (test code, not user input).
 *
 * `timeoutMs` bounds the underlying `psql` process so a query that blocks on a
 * DB lock rejects in seconds instead of hanging the worker indefinitely (A2).
 */
export async function sql(
	query: string,
	opts?: { timeoutMs?: number },
): Promise<string> {
	return psqlExec(query, opts?.timeoutMs);
}

/** Derive the 32-byte ed25519 public key (lowercase hex) from a BIP39 seed. */
export function pubkeyHexFromSeed(seedPhrase: string): string {
	if (!validateMnemonic(seedPhrase)) throw new Error('Invalid seed phrase');
	const seedBuffer = mnemonicToSeedSync(seedPhrase, '');
	const keyMaterial = hmac(sha512, 'ed25519 seed', new Uint8Array(seedBuffer));
	const identity = Ed25519KeyIdentity.fromSecretKey(keyMaterial.slice(0, 32));
	const raw = new Uint8Array(identity.getPublicKey().rawKey);
	return Array.from(raw).map((b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Derive the Ed25519 identity from a BIP39 seed phrase, mirroring the website's
 * `identityFromSeed` (`$lib/utils/identity.ts`) exactly: HMAC-SHA512 keyed with
 * 'ed25519 seed' over `mnemonicToSeedSync(seed, '')`, first 32 bytes as the
 * Ed25519 secret seed. Use this to build a signing identity in Node that the API
 * accepts as the account's real key (the key whose pubkey is stored in
 * account_public_keys by `seedAccountDirect`).
 */
export function identityFromSeedPhrase(seedPhrase: string): Ed25519KeyIdentity {
	if (!validateMnemonic(seedPhrase)) throw new Error('Invalid seed phrase');
	const seedBuffer = mnemonicToSeedSync(seedPhrase, '');
	const keyMaterial = hmac(sha512, 'ed25519 seed', new Uint8Array(seedBuffer));
	return Ed25519KeyIdentity.fromSecretKey(keyMaterial.slice(0, 32));
}

/** Lowercase hex of a Uint8Array. */
function bytesToHex(bytes: Uint8Array): string {
	return Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}

const ED25519_SIGN_CONTEXT = new TextEncoder().encode('decent-cloud');

/**
 * Sign an API request the same way `auth-api.ts:signRequest` does for the
 * website: message = timestamp + nonce + method + path(without query) + body,
 * signed with Ed25519ph (SHA-512 prehash) under the 'decent-cloud' context,
 * using the identity's 32-byte secret seed. Returns the X-* headers and the
 * serialized body to send with a `fetch`/`request` call.
 *
 * Use this (not the src `admin-api.ts` helpers, which depend on the browser-side
 * API_BASE_URL resolution) when a Node spec needs to make real signed API calls.
 */
export function signApiRequest(
	identity: Ed25519KeyIdentity,
	method: string,
	path: string,
	bodyData?: unknown,
): { headers: Record<string, string>; body: string } {
	const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
	const secretSeed = new Uint8Array(identity.getKeyPair().secretKey).slice(0, 32);
	const nonce = crypto.randomUUID();
	const timestampNs = (BigInt(Date.now()) * 1_000_000n).toString();
	let body: string;
	if (typeof bodyData === 'string') body = bodyData;
	else if (bodyData) body = JSON.stringify(bodyData);
	else body = '';
	const pathWithoutQuery = path.split('?')[0];
	const message = new TextEncoder().encode(timestampNs + nonce + method + pathWithoutQuery + body);
	const signature = ed25519ph.sign(message, secretSeed, { context: ED25519_SIGN_CONTEXT });
	return {
		headers: {
			'X-Public-Key': bytesToHex(publicKeyBytes),
			'X-Signature': bytesToHex(signature),
			'X-Timestamp': timestampNs,
			'X-Nonce': nonce,
			'Content-Type': 'application/json',
		},
		body,
	};
}

/**
 * Make a real signed API call from a Node spec. Resolves the API base from the
 * same `api-base.ts` rules (PLAYWRIGHT_API_URL → baseURL port+1 → 59011) so it
 * hits the same stack the browser is driving. Returns the raw Response.
 *
 * The fetch is bounded by `timeoutMs` (AbortSignal.timeout) so a request that
 * stalls — e.g. the API wedged on a DB lock — aborts instead of hanging the
 * test/worker indefinitely (A2: every I/O path needs a timeout).
 */
export async function signedApiCall(
	identity: Ed25519KeyIdentity,
	method: string,
	path: string,
	bodyData?: unknown,
	apiBaseUrl = API_BASE_URL,
	timeoutMs = 15_000,
): Promise<Response> {
	const { headers, body } = signApiRequest(identity, method, path, bodyData);
	return fetch(`${apiBaseUrl}${path}`, {
		method,
		headers,
		body: method === 'GET' || method === 'HEAD' ? undefined : body,
		signal: AbortSignal.timeout(timeoutMs),
	});
}

/** Current time in nanoseconds since epoch. */
export function nowNs(): bigint {
	return BigInt(Date.now()) * 1_000_000n;
}

/**
 * Create a test account directly in the DB, bypassing the ~10-15s UI
 * registration flow (goto /login → revealSeedPhraseOptions → "Generate New"
 * → fill username/email → "Create Account" → "Go to Dashboard").
 *
 * Mirrors exactly what the API's `create_account()` does
 * (`api/src/database/accounts.rs:155`): generates a 16-byte random id for
 * the accounts row, inserts (id, username, email), then inserts the
 * matching account_public_keys row with the ed25519 pubkey derived from
 * the mnemonic. No triggers, no other tables.
 *
 * The returned seed phrase is later injected into localStorage via
 * `context.addInitScript` in the `testAccount` fixture; the client
 * derives the same pubkey and authenticates silently.
 *
 * `username` defaults to the same `test<timestamp><random>` format used
 * by `generateTestUsername()` in auth-helpers for consistency with
 * existing test data.
 */
export async function seedAccountDirect(
	username: string = `test${Date.now()}${Math.floor(Math.random() * 10000)}`,
): Promise<{ username: string; seedPhrase: string }> {
	const seedPhrase = generateMnemonic(128);
	const pubkeyHex = pubkeyHexFromSeed(seedPhrase);
	const email = `${username}@test.example.com`;
	const accountIdHex = randomHex(16);
	const keyIdHex = randomHex(16);

	await sql(`
		INSERT INTO accounts (id, username, email) VALUES (decode('${accountIdHex}', 'hex'), '${username}', '${email}');
		INSERT INTO account_public_keys (id, account_id, public_key) VALUES (decode('${keyIdHex}', 'hex'), decode('${accountIdHex}', 'hex'), decode('${pubkeyHex}', 'hex'));
	`);

	return { username, seedPhrase };
}

/**
 * Delete an account and all dependent rows by username.
 *
 * Most child tables (account_public_keys, billing_settings, etc.) have
 * ON DELETE CASCADE and are removed automatically when the accounts row is
 * deleted. One table has a NO ACTION FK and must be cleaned explicitly
 * first, or the accounts DELETE will fail:
 *   - signature_audit (account_id)
 */
export async function deleteAccountByUsername(username: string): Promise<void> {
	const safeName = username.replace(/'/g, "''");
	await sql(`
		DELETE FROM signature_audit WHERE account_id = (SELECT id FROM accounts WHERE username = '${safeName}');
		DELETE FROM accounts WHERE username = '${safeName}';
	`);
}

/** Random 32-byte lowercase hex string (for contract_id / provider_pubkey). */
export function randomHex(bytes: number): string {
	const buf = Buffer.alloc(bytes);
	for (let i = 0; i < bytes; i++) buf[i] = Math.floor(Math.random() * 256);
	return buf.toString('hex');
}

/**
 * Resolve the 16-byte bytea account id (lowercase hex) for a username. Used by
 * specs that need to seed child rows keyed on accounts.id (contacts, devices,
 * external keys, socials, email-verification tokens, cloud accounts, …).
 */
export async function accountIdHex(username: string): Promise<string> {
	const row = await sql(
		`SELECT encode(id, 'hex') FROM accounts WHERE username = '${username.replace(/'/g, "''")}'`,
	);
	const hex = row.split('\n').map((l) => l.trim()).find((l) => /^[0-9a-f]+$/.test(l));
	if (!hex) throw new Error(`no account id for username ${username}`);
	return hex;
}

/**
 * Seed an `email_verification_tokens` row directly and return the hex token to
 * use in the `/verify-email?token=<hex>` URL.
 *
 * Mirrors `Database::create_email_verification_token`
 * (`api/src/database/accounts.rs:200`): the `token` column is BYTEA (any size;
 * production uses a 16-byte UUID v4), and `created_at`/`expires_at` are BIGINT
 * nanoseconds. Default expiry is 24h, matching production. `used_at` is left
 * NULL so the row is consumable by the verify handler.
 *
 * The verify handler hex-decodes the URL `token` param and looks the row up by
 * exact bytes, so the hex we return is the same hex that goes into the URL.
 */
export async function seedEmailVerificationToken(
	accountHex: string,
	email: string,
	opts?: { expiresInSeconds?: number },
): Promise<string> {
	const tokenHex = randomHex(16);
	const now = nowNs();
	const expiresInNs = BigInt((opts?.expiresInSeconds ?? 86_400) * 1_000_000_000);
	const expiresAt = now + expiresInNs;
	await sql(`
		INSERT INTO email_verification_tokens (token, account_id, email, created_at, expires_at)
		VALUES (
			decode('${tokenHex}', 'hex'),
			decode('${accountHex}', 'hex'),
			'${email.replace(/'/g, "''")}',
			${now},
			${expiresAt}
		)
	`);
	return tokenHex;
}

/**
 * Seed a `recovery_tokens` row directly and return the hex token to use in the
 * `/recover?token=<hex>` URL.
 *
 * Mirrors `Database::create_recovery_token`
 * (`api/src/database/recovery.rs:9`): the `token` column is BYTEA (production
 * uses a 16-byte UUID v4), and `created_at`/`expires_at` are BIGINT ns. Default
 * expiry is 24h (RECOVERY_TOKEN_EXPIRY_NS). `used_at` is left NULL so the row
 * is consumable by `complete_recovery`.
 *
 * The complete handler hex-decodes the URL `token` param (via `decode_hex_path`)
 * and looks the row up by exact bytes, so the hex we return is the same hex
 * that goes into the URL — it MUST be valid hex (even-length, [0-9a-f]+) or the
 * handler rejects with "Invalid recovery token hex: ...".
 */
export async function seedRecoveryToken(
	accountHex: string,
	opts?: { expiresInSeconds?: number },
): Promise<string> {
	const tokenHex = randomHex(16);
	const now = nowNs();
	const expiresInNs = BigInt((opts?.expiresInSeconds ?? 86_400) * 1_000_000_000);
	const expiresAt = now + expiresInNs;
	await sql(`
		INSERT INTO recovery_tokens (token, account_id, created_at, expires_at)
		VALUES (
			decode('${tokenHex}', 'hex'),
			decode('${accountHex}', 'hex'),
			${now},
			${expiresAt}
		)
	`);
	return tokenHex;
}

/** Build the SQL VALUES clause for one contract seed row. See seedContract(). */
export interface ContractSeed {
	/** 32-byte hex ed25519 pubkey of the requester (test user). */
	requesterPubkeyHex: string;
	/** Contract status: 'requested' | 'pending' | 'accepted' | 'provisioning' | 'provisioned' | 'active' | 'cancelled' | 'rejected' | 'failed'. */
	status: string;
	/** Payment status: 'pending' | 'succeeded' | 'failed' | 'refunded'. Default 'succeeded'. */
	paymentStatus?: string;
	/** Currency code. Default 'USD'. */
	currency?: string;
	/** Payment amount in e9s (10^-9 of a token). Default 1 ICP = 1_000_000_000. */
	paymentAmountE9s?: number | string;
	/** Duration in hours. Default 1. */
	durationHours?: number;
	/** Optional offering_id. Default 'compute-001' (from seed_data.sql). */
	offeringId?: string;
	/** Optional provider 32-byte hex pubkey. Random by default. */
	providerPubkeyHex?: string;
	/** Payment method. Default 'test'. Use 'stripe' to exercise the refund gate. */
	paymentMethod?: string;
	/** Stripe payment intent id (required when paymentMethod='stripe' and the refund gate runs). */
	stripePaymentIntentId?: string;
}

/**
 * Insert a contract_sign_requests row for the given requester. Returns the
 * lowercase hex contract_id of the new row.
 *
 * `created_at_ns` is set to a stable per-call value (current time in ns), so
 * tests ordering contracts by created_at_ns DESC see insertion order.
 */
export async function seedContract(seed: ContractSeed): Promise<string> {
	const contractId = randomHex(32);
	const providerPubkey = seed.providerPubkeyHex ?? randomHex(32);
	const currency = seed.currency ?? 'USD';
	const paymentAmount = seed.paymentAmountE9s ?? 1_000_000_000;
	const durationHours = seed.durationHours ?? 1;
	const offeringId = seed.offeringId ?? 'compute-001';
	const paymentStatus = seed.paymentStatus ?? 'succeeded';
	const paymentMethod = seed.paymentMethod ?? 'test';
	const createdAt = nowNs().toString();
	const sshPubkey = 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAItestdata test@example.com';
	const contact = 'email:test@example.com';
	const memo = `E2E seed contract ${contractId.slice(0, 8)}`;

	// The schema requires offering_id to reference an existing offering for the
	// JOIN that fetches offering_name, but the FK is on (offering_id, provider_pubkey)
	// in provider_offerings. To stay decoupled from marketplace state we insert
	// with offering_id='1' which is part of seed_data.sql.
	await sql(`
		INSERT INTO contract_sign_requests (
			contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact,
			provider_pubkey, offering_id, payment_amount_e9s, duration_hours,
			original_duration_hours, request_memo, created_at_ns, status,
			status_updated_at_ns, currency, payment_method, payment_status${seed.stripePaymentIntentId ? ', stripe_payment_intent_id' : ''}
		) VALUES (
			decode('${contractId}', 'hex'),
			decode('${seed.requesterPubkeyHex}', 'hex'),
			'${sshPubkey.replace(/'/g, "''")}',
			'${contact.replace(/'/g, "''")}',
			decode('${providerPubkey}', 'hex'),
			'${offeringId}',
			${paymentAmount},
			${durationHours},
			${durationHours},
			'${memo.replace(/'/g, "''")}',
			${createdAt},
			'${seed.status}',
			${createdAt},
			'${currency}',
			'${paymentMethod}',
			'${paymentStatus}'${seed.stripePaymentIntentId ? `, '${seed.stripePaymentIntentId.replace(/'/g, "''")}'` : ''}
		)
	`);
	return contractId;
}

/**
 * Insert a refund_requests row directly (bypasses the gate). Used by admin
 * panel e2e tests to seed the pending/approved/declined state the UI renders,
 * without triggering real Stripe API calls (which the cancel path would do
 * when the warm stack has STRIPE_SECRET_KEY configured).
 *
 * Returns the new row id as a string.
 *
 * Mirrors the columns `process_gated_refund` writes. `UNIQUE(contract_id,
 * reason)` is respected — each (contract, reason) pair can only have one row.
 */
export async function seedRefundRequest(opts: {
	contractIdHex: string;
	requesterPubkeyHex: string;
	refundAmountE9s: number | string;
	reason: string;
	status?: string;
	userLatestPaymentE9s?: number | string;
	capExceeded?: boolean;
	paymentIntentId?: string;
	currency?: string;
}): Promise<string> {
	const status = opts.status ?? 'pending';
	const userLatest = opts.userLatestPaymentE9s ?? 0;
	const capExceeded = opts.capExceeded ?? false;
	const paymentIntentId = opts.paymentIntentId ?? 'pi_test_seed';
	const currency = opts.currency ?? 'USD';
	const createdAt = nowNs().toString();
	const idempotencyKey = `seed-${opts.contractIdHex.slice(0, 16)}-${opts.reason}`;

	const out = await psqlExec(`INSERT INTO refund_requests (
			contract_id, requester_pubkey, refund_amount_e9s, reason, status,
			user_latest_payment_e9s, cap_exceeded, payment_intent_id, currency,
			idempotency_key, created_at_ns
		) VALUES (
			decode('${opts.contractIdHex}', 'hex'),
			decode('${opts.requesterPubkeyHex}', 'hex'),
			${opts.refundAmountE9s},
			'${opts.reason}',
			'${status}',
			${userLatest},
			${capExceeded},
			'${paymentIntentId}',
			'${currency}',
			'${idempotencyKey}',
			${createdAt}
		) RETURNING id`);
	return out.split('\n')[0];
}

/** Delete contracts for a requester pubkey (cleanup).
 *
 * contract_sign_requests is referenced by several tables without ON DELETE
 * CASCADE (contract_events, contract_usage, contract_usage_events,
 * contract_health_checks, invoices). We must delete those first or the
 * DELETE fails with an FK violation. Combined into one psql invocation to
 * minimise process spawn overhead under parallel workers.
 */
export async function deleteContractsForRequester(requesterPubkeyHex: string): Promise<void> {
	await sql(`
		DELETE FROM refund_requests WHERE requester_pubkey = decode('${requesterPubkeyHex}', 'hex');
		DELETE FROM contract_events
			WHERE contract_id IN (SELECT contract_id FROM contract_sign_requests WHERE requester_pubkey = decode('${requesterPubkeyHex}', 'hex'));
		DELETE FROM contract_usage_events
			WHERE contract_id IN (SELECT contract_id FROM contract_sign_requests WHERE requester_pubkey = decode('${requesterPubkeyHex}', 'hex'));
		DELETE FROM contract_usage
			WHERE contract_id IN (SELECT contract_id FROM contract_sign_requests WHERE requester_pubkey = decode('${requesterPubkeyHex}', 'hex'));
		DELETE FROM contract_health_checks
			WHERE contract_id IN (SELECT contract_id FROM contract_sign_requests WHERE requester_pubkey = decode('${requesterPubkeyHex}', 'hex'));
		DELETE FROM invoices
			WHERE contract_id IN (SELECT contract_id FROM contract_sign_requests WHERE requester_pubkey = decode('${requesterPubkeyHex}', 'hex'));
		DELETE FROM contract_sign_requests WHERE requester_pubkey = decode('${requesterPubkeyHex}', 'hex');
	`);
}

/**
 * Delete contracts where the account is the PROVIDER (cleanup for provider-side
 * seeding, e.g. provider-earnings populated state). Mirror of
 * `deleteContractsForRequester` keyed on `provider_pubkey` instead.
 *
 * Same NO-ACTION FK child tables must be cleared first or the DELETE fails.
 */
export async function deleteContractsByProvider(providerPubkeyHex: string): Promise<void> {
	await sql(`
		DELETE FROM contract_events
			WHERE contract_id IN (SELECT contract_id FROM contract_sign_requests WHERE provider_pubkey = decode('${providerPubkeyHex}', 'hex'));
		DELETE FROM contract_usage_events
			WHERE contract_id IN (SELECT contract_id FROM contract_sign_requests WHERE provider_pubkey = decode('${providerPubkeyHex}', 'hex'));
		DELETE FROM contract_usage
			WHERE contract_id IN (SELECT contract_id FROM contract_sign_requests WHERE provider_pubkey = decode('${providerPubkeyHex}', 'hex'));
		DELETE FROM contract_health_checks
			WHERE contract_id IN (SELECT contract_id FROM contract_sign_requests WHERE provider_pubkey = decode('${providerPubkeyHex}', 'hex'));
		DELETE FROM invoices
			WHERE contract_id IN (SELECT contract_id FROM contract_sign_requests WHERE provider_pubkey = decode('${providerPubkeyHex}', 'hex'));
		DELETE FROM contract_sign_requests WHERE provider_pubkey = decode('${providerPubkeyHex}', 'hex');
	`);
}

/**
 * Delete agent pools and the auto-created provider_registrations row for a
 * provider pubkey (cleanup for the agent-pool create flow).
 *
 * `create_agent_pool` auto-creates the `provider_registrations` FK row
 * (agent_pools.rs:~205, `INSERT ... ON CONFLICT DO NOTHING`), which is NOT
 * cascaded by account teardown (pubkey is bytea, not an accounts FK). So both
 * must be removed explicitly. offerings/delegations referencing a pool are
 * detached first to avoid blocking the pool delete with a NO-ACTION FK.
 */
export async function deleteAgentPoolsByProvider(providerPubkeyHex: string): Promise<void> {
	await sql(`
		UPDATE provider_offerings SET agent_pool_id = NULL
			WHERE agent_pool_id IN (SELECT pool_id FROM agent_pools WHERE provider_pubkey = decode('${providerPubkeyHex}', 'hex'));
		DELETE FROM provider_agent_delegations
			WHERE pool_id IN (SELECT pool_id FROM agent_pools WHERE provider_pubkey = decode('${providerPubkeyHex}', 'hex'));
		DELETE FROM agent_pools WHERE provider_pubkey = decode('${providerPubkeyHex}', 'hex');
		DELETE FROM provider_registrations WHERE pubkey = decode('${providerPubkeyHex}', 'hex');
	`);
}

/**
 * Delete the provider profile (+ onboarding tracking) for a pubkey. Used to
 * clean up the become-provider onboarding submit flow, whose signed PUT upserts
 * into provider_profiles (support_hours, channels, regions, payment_methods…).
 * provider_profiles_contacts cascades on the profile delete.
 */
export async function deleteProviderProfileByPubkey(providerPubkeyHex: string): Promise<void> {
	await sql(`
		DELETE FROM provider_onboarding WHERE provider_pubkey = decode('${providerPubkeyHex}', 'hex');
		DELETE FROM provider_profiles WHERE pubkey = decode('${providerPubkeyHex}', 'hex');
	`);
}

/**
 * Flip `email_verified = true` on the accounts row that owns the given public
 * key. Mirrors the post-verify state of `Database::verify_email_token`
 * (`api/src/database/accounts.rs:285`) without requiring a verification token,
 * so specs that need a verified user for realistic dashboard rendering (e.g.
 * route-audit, rent-flow) can set it up in one SQL round-trip.
 */
export async function verifyAccountEmail(pubkeyHex: string): Promise<void> {
	await sql(`
		UPDATE accounts SET email_verified = true
		WHERE id = (
			SELECT account_id FROM account_public_keys
			WHERE public_key = decode('${pubkeyHex}', 'hex')
		)
	`);
}

/**
 * Flip `email_verified = false` on the accounts row that owns the given public
 * key — the inverse of `verifyAccountEmail`. Needed by specs whose premise is
 * that the user starts UNVERIFIED (e.g. rent-email-verification-gate): under
 * serial `--workers 1` mode the shared `testAccount` pubkey can be left
 * verified by an earlier spec's `beforeAll` (rent-dialog-keyboard calls
 * `verifyAccountEmail`), so a spec that asserts the unverified UI must own its
 * precondition explicitly instead of relying on the `seedAccountDirect` default.
 */
export async function unverifyAccountEmail(pubkeyHex: string): Promise<void> {
	await sql(`
		UPDATE accounts SET email_verified = false
		WHERE id = (
			SELECT account_id FROM account_public_keys
			WHERE public_key = decode('${pubkeyHex}', 'hex')
		)
	`);
}

/** Remove all saved offerings for a user (cleanup helper shared by specs). */
export async function deleteSavedOfferingsForUser(userPubkeyHex: string): Promise<void> {
	await sql(`DELETE FROM saved_offerings WHERE user_pubkey = decode('${userPubkeyHex}', 'hex')`);
}

/** Overrides for seedOffering(). All optional — sensible defaults provided. */
export interface OfferingSeedOverrides {
	/** Stable per-test offering_id. Default: `test-<timestamp>`. */
	offeringId?: string;
	/** Initial visibility. Default 'public'. */
	visibility?: string;
	/** Initial stock_status. Default 'in_stock'. */
	stockStatus?: string;
	/** Optional name override. Default 'Test Offering'. */
	name?: string;
	/** Currency code for the offering's price. Default 'USD'. Stripe is the
	 * sole payment rail, so offerings should be priced in a Stripe-supported
	 * currency; the rental dialog's Stripe (Credit Card) path requires fiat. */
	currency?: string;
	/** offering_source column. Default unset (NULL → treated as a normal provider
	 * offering, which is offline without an agent pool). Set 'self_provisioned'
	 * to make the offering always online (compute_provider_online_status treats
	 * self_provisioned as always-online, bypassing pool/agent requirements). */
	offeringSource?: string;
	/** post_provision_script (a.k.a. "setup recipe"). When set, the marketplace
	 * OfferingStatusBadge exposes a tooltip ("Has setup recipe") and the row
	 * matches the recipes filter. Default unset (no recipe). */
	postProvisionScript?: string;
	/** is_subscription flag + interval. When true, the badge exposes a
	 * "Subscription" tooltip. Default false (not a subscription). */
	isSubscription?: boolean;
	subscriptionIntervalDays?: number;
	/** product_type column. Default 'compute'. Set 'gpu' / 'storage' / 'network'
	 * to exercise the marketplace type filter + the DSL `type:` field. */
	productType?: string;
	/** monthly_price column (fiat). Default 25.0. Set to known distinct values
	 * so price-filter DSL assertions (`price:<=N`) are deterministic. */
	monthlyPrice?: number;
}

/**
 * Insert a provider_offerings row and return the numeric BIGSERIAL id (as a
 * string from psql). Only the NOT NULL columns are populated; bytes use
 * decode(...,'hex'). Signed PUTs are accepted because the row's pubkey
 * matches the caller's identity.
 *
 * Shared by specs that need to seed offering rows with different column
 * values (visibility, stock_status, name) without duplicating the INSERT SQL.
 */
export async function seedOffering(pubkeyHex: string, overrides?: OfferingSeedOverrides): Promise<string> {
	const offeringId = overrides?.offeringId ?? `test-${Date.now()}`;
	const visibility = overrides?.visibility ?? 'public';
	const stockStatus = overrides?.stockStatus ?? 'in_stock';
	const currency = overrides?.currency ?? 'USD';
	const productType = overrides?.productType ?? 'compute';
	const monthlyPrice = overrides?.monthlyPrice ?? 25.0;
	const name = (overrides?.name ?? 'Test Offering').replace(/'/g, "''");
	const createdAt = nowNs().toString();
	const sourceCol = overrides?.offeringSource ? ', offering_source' : '';
	const sourceVal = overrides?.offeringSource ? `, '${overrides.offeringSource.replace(/'/g, "''")}'` : '';
	const recipeCol = overrides?.postProvisionScript ? ', post_provision_script' : '';
	const recipeVal = overrides?.postProvisionScript ? `, '${overrides.postProvisionScript.replace(/'/g, "''")}'` : '';
	const subCol = overrides?.isSubscription ? ', is_subscription, subscription_interval_days' : '';
	const subVal = overrides?.isSubscription ? `, true, ${overrides.subscriptionIntervalDays ?? 30}` : '';
	const result = await sql(`
		INSERT INTO provider_offerings (
			pubkey, offering_id, offer_name, currency, monthly_price,
			visibility, product_type, billing_interval, stock_status,
			datacenter_country, datacenter_city, created_at_ns${sourceCol}${recipeCol}${subCol}
		) VALUES (
			decode('${pubkeyHex}', 'hex'),
			'${offeringId}',
			'${name}',
			'${currency}', ${monthlyPrice},
			'${visibility}', '${productType}', 'monthly', '${stockStatus}',
			'US', 'New York', ${createdAt}${sourceVal}${recipeVal}${subVal}
		)
		RETURNING id
	`);
	const numericId = result.split('\n').map((l) => l.trim()).find((l) => /^\d+$/.test(l));
	if (!numericId) throw new Error(`seedOffering did not RETURN a numeric id; got: ${result}`);
	return numericId;
}

/**
 * Seed a rentable marketplace offering under a fresh random non-example provider
 * pubkey. Uses offering_source='self_provisioned' so compute_provider_online_status
 * marks it online without requiring an agent pool or provider_agent_status row.
 * Returns identifiers for navigation + cleanup.
 *
 * The random pubkey is not a registered account, which is fine: provider_offerings
 * has no FK on pubkey and the marketplace query LEFT JOINs accounts (owner_username
 * is null). is_example is false (random pubkey != example provider pubkey).
 */
export async function seedRentableOffering(overrides?: OfferingSeedOverrides): Promise<{
	providerPubkeyHex: string;
	offeringNumericId: string;
	offeringId: string;
	offeringName: string;
}> {
	const providerPubkeyHex = randomHex(32);
	const offeringId = overrides?.offeringId ?? `rentable-${Date.now()}-${Math.floor(Math.random() * 100000)}`;
	const offeringName = overrides?.name ?? 'E2E Rentable Offering';
	const offeringNumericId = await seedOffering(providerPubkeyHex, {
		...overrides,
		offeringId,
		name: offeringName,
		offeringSource: overrides?.offeringSource ?? 'self_provisioned',
	});
	return { providerPubkeyHex, offeringNumericId, offeringId, offeringName };
}

/** Delete all offerings for a provider pubkey (cleanup). */
export async function deleteOfferingsByProvider(pubkeyHex: string): Promise<void> {
	await sql(`DELETE FROM provider_offerings WHERE pubkey = decode('${pubkeyHex}', 'hex')`);
}

/**
 * The placeholder "example provider" pubkey (a readable ASCII string, NOT a real
 * ed25519 key). The marketplace query flags any offering under this pubkey with
 * `is_example = true` — the "demo" filter the showDemoOfferings toggle controls.
 *
 * Migration 053 deleted the demo seed rows (offerings, profile, registration)
 * under this pubkey, so the marketplace is honestly empty after the product
 * pivot (PRODUCT-DIRECTION.md F2). The runtime exclusion filter is intentionally
 * RETAINED, so specs exercising the demo/offline UI self-seed an offering under
 * this pubkey to make `is_example` true. NEVER use this as a real provider.
 */
export const EXAMPLE_PROVIDER_PUBKEY_HEX =
	'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';

/**
 * Insert a minimal `provider_registrations` row so child tables whose FK targets
 * it (`provider_offering_sla_targets.provider_pubkey`, `agent_pools`, …) can be
 * seeded. Mirrors the test-only INSERT used across `api/src/database` tests.
 *
 * Idempotent (`ON CONFLICT DO NOTHING`) so several specs / parallel workers
 * seeding the same pubkey (e.g. EXAMPLE_PROVIDER_PUBKEY_HEX) never collide.
 */
export async function seedProviderRegistration(pubkeyHex: string): Promise<void> {
	await sql(`
		INSERT INTO provider_registrations (pubkey, signature, created_at_ns)
		VALUES (decode('${pubkeyHex}', 'hex'), '\\x00', 0)
		ON CONFLICT (pubkey) DO NOTHING
	`);
}

/**
 * Delete a single `provider_offerings` row by its numeric BIGSERIAL id. Surgical
 * cleanup for specs that seed under a SHARED pubkey (e.g.
 * EXAMPLE_PROVIDER_PUBKEY_HEX) where `deleteOfferingsByProvider` would nuke other
 * workers' rows. Cascades to `provider_offering_sla_targets` / sli reports via
 * the `offering_id` FK.
 */
export async function deleteOfferingById(numericId: string | number): Promise<void> {
	await sql(`DELETE FROM provider_offerings WHERE id = ${numericId}`);
}

/** Handle returned by seedMarketplaceOffering for navigation + per-row cleanup. */
export interface MarketplaceOfferingHandle {
	/** 32-byte hex pubkey the offering was seeded under (example pubkey when demo). */
	providerPubkeyHex: string;
	/** Numeric BIGSERIAL id of the provider_offerings row (for /marketplace/<id>). */
	offeringNumericId: string;
	/** String offering_id. */
	offeringId: string;
}

/**
 * Seed a public marketplace offering for the browsing/sort/offline/SLA specs
 * broken by the drop-demos pivot (migration 053). Builds on `seedOffering` along
 * the two axes those specs vary:
 *   - `isExample`: seed under EXAMPLE_PROVIDER_PUBKEY_HEX so the marketplace
 *     query flags the row `is_example` (the "demo" filter). Default false → a
 *     fresh random 32-byte pubkey (a real, non-demo offering).
 *   - `online`: set `offering_source='self_provisioned'` so
 *     `compute_provider_online_status` marks the provider online without an
 *     agent pool. Default false → a plain offering with no pool, which is
 *     OFFLINE and therefore hidden by the default marketplace view
 *     (`showOfflineOfferings=false`).
 *
 * Returns identifiers for navigation and surgical cleanup
 * (`deleteOfferingById`). Prefer per-id cleanup so specs sharing the example
 * pubkey don't delete each other's rows under parallel workers.
 */
export async function seedMarketplaceOffering(
	opts?: OfferingSeedOverrides & { isExample?: boolean; online?: boolean },
): Promise<MarketplaceOfferingHandle> {
	const providerPubkeyHex = opts?.isExample ? EXAMPLE_PROVIDER_PUBKEY_HEX : randomHex(32);
	const offeringId = opts?.offeringId ?? `mkt-${Date.now()}-${Math.floor(Math.random() * 100000)}`;
	const offeringNumericId = await seedOffering(providerPubkeyHex, {
		...opts,
		offeringId,
		// `online` wins over an explicit offeringSource so the two can't conflict.
		offeringSource: opts?.online ? 'self_provisioned' : opts?.offeringSource,
	});
	return { providerPubkeyHex, offeringNumericId, offeringId };
}

/**
 * Seed for a REAL tenant→provider rental flow (rent-flow.spec.ts).
 *
 * `seedRentableOffering` alone makes an offering VISIBLE+ONLINE in the marketplace
 * (self_provisioned → provider_online=true), but the API's `create_rental_request`
 * for a self_provisioned offering ALSO reserves a `cloud_resources` row
 * (listing_mode='marketplace', status='running', contract_id IS NULL) and rolls
 * the whole insert back with "out of stock" if none is available. So a genuine
 * UI rental that lands a real contract needs the full chain this helper wires:
 *   provider account → cloud_account → cloud_resource(s) → self_provisioned offering.
 *
 * The offering is under a RANDOM provider pubkey (not the testAccount pubkey), so
 * the rental is a real tenant→provider request (not a degenerate self-rental).
 * `resourceCount` defaults to 4 so several serial rentals (rent+cancel from both
 * the list and the detail page) don't exhaust stock. `currency` defaults to 'USD'
 * so the rental dialog selects the Stripe (Credit Card) path — the only payment
 * path with no pre-submit wallet/config guard, and the one whose external boundary
 * (Stripe) is permitted to be mocked in e2e.
 */
export interface RentableWithResourceSeed {
	/** Random 32-byte hex pubkey of the (synthetic) provider. */
	providerPubkeyHex: string;
	/** 16-byte hex id of the synthetic provider accounts row (for cleanup). */
	providerAccountIdHex: string;
	/** Numeric BIGSERIAL id of the provider_offerings row (for /marketplace/<id>). */
	offeringNumericId: string;
	/** String offering_id (e.g. rentflow-<tag>). */
	offeringId: string;
	/** Human-readable offering name (for marketplace matching). */
	offeringName: string;
	/** cloud_resources.external_id values seeded (for cleanup). */
	resourceExternalIds: string[];
}

export async function seedRentableWithResource(opts?: {
	name?: string;
	currency?: string;
	resourceCount?: number;
	/**
	 * Whether the seeded provider auto-accepts wallet-paid rentals. Inserts an
	 * explicit `provider_profiles` row so the decision is INTENTIONAL rather
	 * than relying on the schema default (which is `true` for a missing row).
	 *
	 * - `false` (default): the contract lands at `requested` (manual-review
	 *   path). Required by specs that assert the pre-service full-refund wallet
	 *   math (cancel at `requested` → `provisioning_completed_at_ns` IS NULL →
	 *   full principal refunded). rent-flow + rent-dialog-keyboard use this.
	 * - `true`: `try_auto_accept_contract` advances the contract past
	 *   `requested` to `accepted` (the wallet-auto-accept path). Used by
	 *   rent-wallet-auto-accept.spec.ts to prove wallet-paid rentals no longer
	 *   get stuck at `requested` (the marketplace buy-flow bug).
	 */
	autoAcceptRentals?: boolean;
}): Promise<RentableWithResourceSeed> {
	const providerPubkeyHex = randomHex(32);
	const providerAccountIdHex = randomHex(16);
	const tag = randomHex(4);
	const cloudAccountName = `e2e-ca-${tag}`;
	const providerUsername = `e2eprov-${tag}`;
	const providerEmail = `${providerUsername}@test.example.com`;
	const offeringId = `rentflow-${tag}`;
	const offeringName = opts?.name ?? 'E2E Rent Flow Offering';
	const currency = opts?.currency ?? 'USD';
	const resourceCount = opts?.resourceCount ?? 4;
	const autoAcceptRentals = opts?.autoAcceptRentals ?? false;

	// Provider accounts row — cloud_accounts.account_id has an FK to accounts.id (bytea).
	await sql(`
		INSERT INTO accounts (id, username, email)
		VALUES (decode('${providerAccountIdHex}', 'hex'), '${providerUsername}', '${providerEmail}')
	`);

	// cloud_account (id is auto-gen uuid); RETURNING id gives the uuid for the resources.
	// psql emits a trailing "INSERT 0 1" command tag even with --tuples-only, so split
	// and pick the uuid line (same trick seedOffering uses for its RETURNING id).
	const cloudAccountRaw = await sql(`
		INSERT INTO cloud_accounts (account_id, backend_type, name, credentials_encrypted)
		VALUES (decode('${providerAccountIdHex}', 'hex'), 'hetzner', '${cloudAccountName}', 'encrypted')
		RETURNING id
	`);
	const cloudAccountUuid = cloudAccountRaw
		.split('\n')
		.map((l) => l.trim())
		.find((l) => /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/.test(l));
	if (!cloudAccountUuid) {
		throw new Error(
			`seedRentableWithResource: cloud_accounts RETURNING id did not yield a uuid; got: ${cloudAccountRaw}`,
		);
	}

	// self_provisioned public offering under the random provider pubkey.
	const offeringNumericId = await seedOffering(providerPubkeyHex, {
		offeringId,
		name: offeringName,
		currency,
		offeringSource: 'self_provisioned',
		stockStatus: 'in_stock',
	});

	// Explicit provider_profiles row so the auto-accept decision is intentional.
	// Without a row, get_provider_auto_accept_rentals falls back to the schema
	// default (TRUE) — which would auto-advance wallet-paid rentals past
	// `requested`. Inserting the row pins the behavior the caller asked for.
	// (Matches the insert shape used by provider-accept-reject.spec.ts.)
	await sql(`
		INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns, auto_accept_rentals)
		VALUES (decode('${providerPubkeyHex}', 'hex'), '${offeringName}', 'v1', '1', ${nowNs()}, ${autoAcceptRentals})
		ON CONFLICT (pubkey) DO UPDATE SET auto_accept_rentals = ${autoAcceptRentals}
	`);

	// Reservable cloud_resources linked to this offering, listed on the marketplace.
	const resourceExternalIds: string[] = [];
	for (let i = 0; i < resourceCount; i++) {
		const ext = `e2e-res-${tag}-${i}`;
		resourceExternalIds.push(ext);
		await sql(`
			INSERT INTO cloud_resources (
				cloud_account_id, external_id, name, server_type, location,
				image, ssh_pubkey, status, offering_id, listing_mode
			) VALUES (
				'${cloudAccountUuid}', '${ext}', 'e2e-vm-${tag}-${i}', 'cx22', 'nbg1',
				'ubuntu-24.04', 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIe2eowner', 'running',
				${offeringNumericId}, 'marketplace'
			)
		`);
	}

	return {
		providerPubkeyHex,
		providerAccountIdHex,
		offeringNumericId,
		offeringId,
		offeringName,
		resourceExternalIds,
	};
}

/**
 * Tear down a `seedRentableWithResource` seed. Clears resource→contract links
 * first (cloud_resources.contract_id has a NO-ACTION FK to contract_sign_requests,
 * so an un-cancelled contract would otherwise block deletion), then removes the
 * resources, the offering, and the provider account (cascades cloud_account).
 *
 * The rental CONTRACTS themselves belong to the requester (testAccount) pubkey,
 * not the provider — clean those with `deleteContractsForRequester(requesterHex)`.
 */
export async function cleanupRentableWithResource(seed: RentableWithResourceSeed): Promise<void> {
	const extList = seed.resourceExternalIds.map((e) => `'${e}'`).join(',');
	await sql(`
		UPDATE cloud_resources SET contract_id = NULL WHERE external_id IN (${extList});
		DELETE FROM cloud_resources WHERE external_id IN (${extList});
		DELETE FROM provider_offerings WHERE pubkey = decode('${seed.providerPubkeyHex}', 'hex');
		DELETE FROM provider_profiles WHERE pubkey = decode('${seed.providerPubkeyHex}', 'hex');
		DELETE FROM accounts WHERE id = decode('${seed.providerAccountIdHex}', 'hex');
	`);
}

/**
 * Seed a `cloud_accounts` row owned by `accountHex` (the testAccount's bytea
 * account id) and return its UUID id. Mirrors the INSERT shape
 * `seedRentableWithResource` uses, but under a GIVEN account_id so the row
 * shows up on THAT user's `/dashboard/cloud/accounts` page (the list endpoint
 * filters by the caller's account_id).
 *
 * Use this to assert the populated cloud-accounts list + the disconnect flow
 * without real Hetzner/Proxmox credentials. `backendType` defaults to
 * 'hetzner'; `credentials_encrypted` is a placeholder (the list page never
 * decrypts it). Cleanup is `deleteCloudAccountsForAccount(accountHex)`.
 */
export async function seedCloudAccount(
	accountHex: string,
	opts?: { name?: string; backendType?: string },
): Promise<string> {
	const name = (opts?.name ?? `E2E Cloud Acct ${randomHex(4)}`).replace(/'/g, "''");
	const backendType = opts?.backendType ?? 'hetzner';
	const raw = await sql(`
		INSERT INTO cloud_accounts (account_id, backend_type, name, credentials_encrypted)
		VALUES (decode('${accountHex}', 'hex'), '${backendType}', '${name}', 'e2e-placeholder-encrypted')
		RETURNING id
	`);
	const uuid = raw.split('\n').map((l) => l.trim()).find((l) => /[0-9a-f]{8}-[0-9a-f]{4}/.test(l));
	if (!uuid) throw new Error(`seedCloudAccount did not RETURN a uuid; got: ${raw}`);
	return uuid;
}

/** Remove all cloud_accounts for an account id (cleanup). Cascades cloud_resources. */
export async function deleteCloudAccountsForAccount(accountHex: string): Promise<void> {
	await sql(`DELETE FROM cloud_accounts WHERE account_id = decode('${accountHex}', 'hex')`);
}
