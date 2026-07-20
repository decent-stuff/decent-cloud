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
import { mnemonicToSeedSync, validateMnemonic } from 'bip39';
import { hmac } from '@noble/hashes/hmac';
import { sha512 } from '@noble/hashes/sha512';

const execFileAsync = promisify(execFile);

const DATABASE_URL = process.env.DATABASE_URL || 'postgres://test:test@postgres:5432/test';

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
 * Run a SQL command via psql; returns trimmed stdout.
 * Throws on non-zero exit. Use $1/$2/... placeholders in `sql` and pass
 * values in `params` — they're bound safely via psql --v variables.
 *
 * For tests we only need INSERTs/UPDATEs with bytea literals; building them
 * from hex with `decode(..., 'hex')` is safe against SQL injection because
 * callers control all inputs (test code, not user input).
 */
export async function sql(query: string): Promise<string> {
	const { args, env } = psqlArgs();
	const { stdout } = await execFileAsync('psql', [...args, '--command', query], { env });
	return stdout.trim();
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

/** Current time in nanoseconds since epoch. */
export function nowNs(): bigint {
	return BigInt(Date.now()) * 1_000_000n;
}

/** Random 32-byte lowercase hex string (for contract_id / provider_pubkey). */
export function randomHex(bytes: number): string {
	const buf = Buffer.alloc(bytes);
	for (let i = 0; i < bytes; i++) buf[i] = Math.floor(Math.random() * 256);
	return buf.toString('hex');
}

/** Build the SQL VALUES clause for one contract seed row. See seedContract(). */
export interface ContractSeed {
	/** 32-byte hex ed25519 pubkey of the requester (test user). */
	requesterPubkeyHex: string;
	/** Contract status: 'requested' | 'pending' | 'accepted' | 'provisioning' | 'provisioned' | 'active' | 'cancelled' | 'rejected' | 'failed'. */
	status: string;
	/** Payment status: 'pending' | 'succeeded' | 'failed' | 'refunded'. Default 'succeeded'. */
	paymentStatus?: string;
	/** Currency code. Default 'ICP'. */
	currency?: string;
	/** Payment amount in e9s (10^-9 of a token). Default 1 ICP = 1_000_000_000. */
	paymentAmountE9s?: number | string;
	/** Duration in hours. Default 1. */
	durationHours?: number;
	/** Optional offering_id. Default 'compute-001' (from seed_data.sql). */
	offeringId?: string;
	/** Optional provider 32-byte hex pubkey. Random by default. */
	providerPubkeyHex?: string;
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
	const currency = seed.currency ?? 'ICP';
	const paymentAmount = seed.paymentAmountE9s ?? 1_000_000_000;
	const durationHours = seed.durationHours ?? 1;
	const offeringId = seed.offeringId ?? 'compute-001';
	const paymentStatus = seed.paymentStatus ?? 'succeeded';
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
			status_updated_at_ns, currency, payment_method, payment_status
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
			'icpay',
			'${paymentStatus}'
		)
	`);
	return contractId;
}

/** Insert a token_transfers row. Returns the new row id. */
export async function seedTransfer(opts: {
	fromAccount: string;
	toAccount: string;
	amountE9s?: number | string;
	feeE9s?: number | string;
	memo?: string;
}): Promise<string> {
	const amount = opts.amountE9s ?? 1_000_000_000;
	const fee = opts.feeE9s ?? 10_000;
	const memo = opts.memo ? `'${opts.memo.replace(/'/g, "''")}'` : 'NULL';
	const createdAt = nowNs().toString();
	const { stdout } = await execFileAsync('psql', [
		...psqlArgs().args,
		'--command',
		`INSERT INTO token_transfers (from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns) VALUES ('${opts.fromAccount}', '${opts.toAccount}', ${amount}, ${fee}, ${memo}, ${createdAt}) RETURNING id`,
	], { env: psqlArgs().env });
	return stdout.trim();
}

/** Delete contracts for a requester pubkey (cleanup).
 *
 * contract_sign_requests is referenced by several tables without ON DELETE
 * CASCADE (contract_events, contract_usage, contract_usage_events,
 * contract_health_checks, invoices). We must delete those first or the
 * DELETE fails with an FK violation.
 */
export async function deleteContractsForRequester(requesterPubkeyHex: string): Promise<void> {
	// Child tables that reference contract_id without CASCADE
	const childTables = [
		'contract_events',
		'contract_usage_events',
		'contract_usage',
		'contract_health_checks',
		'invoices',
	];
	for (const table of childTables) {
		await sql(`
			DELETE FROM ${table}
			WHERE contract_id IN (
				SELECT contract_id FROM contract_sign_requests
				WHERE requester_pubkey = decode('${requesterPubkeyHex}', 'hex')
			)
		`);
	}
	await sql(`DELETE FROM contract_sign_requests WHERE requester_pubkey = decode('${requesterPubkeyHex}', 'hex')`);
}

/** Delete transfers where account is sender or receiver. */
export async function deleteTransfersForAccount(account: string): Promise<void> {
	await sql(`DELETE FROM token_transfers WHERE from_account = '${account.replace(/'/g, "''")}' OR to_account = '${account.replace(/'/g, "''")}'`);
}
