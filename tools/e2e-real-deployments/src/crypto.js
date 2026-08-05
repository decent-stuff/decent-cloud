// Request signing for the decent-cloud API — mirrors the website's auth-api.ts
// and the Rust verifier (api/src/auth.rs + common/src/api_auth.rs) EXACTLY.
//
// Canonical signed message (raw byte concat, no separators):
//   timestamp_ns || nonce || METHOD || fullPath || body
// where fullPath includes the `/api/v1` prefix and excludes the query string.
// Signature: Ed25519ph (SHA-512 prehash) with context b"decent-cloud".
//
// Identity derivation (matches website identityFromSeed):
//   seed = BIP39 mnemonicToSeed(phrase, "")
//   keyMaterial = HMAC-SHA512(key=b"ed25519 seed", msg=seed)
//   secretKey   = keyMaterial[0:32]
//   publicKey   = ed25519ph.getPublicKey(secretKey)

import { loadModule } from './deps.js';

let _ed, _hmac, _sha512, _mnemonic;
async function libs() {
	if (!_ed) {
		[_ed, _hmac, _sha512, _mnemonic] = await Promise.all([
			loadModule('@noble/curves/ed25519'),
			loadModule('@noble/hashes/hmac'),
			loadModule('@noble/hashes/sha512'),
			loadModule('bip39'),
		]);
	}
	return { ed: _ed.ed25519ph, hmac: _hmac.hmac, sha512: _sha512.sha512, mnemonicToSeedSync: _mnemonic.mnemonicToSeedSync };
}

const SIGN_CONTEXT = new TextEncoder().encode('decent-cloud');

/** @typedef {{secretKey: Uint8Array, publicKey: Uint8Array}} Identity */

/** Convert bytes to lowercase hex. */
function bytesToHex(bytes) {
	let out = '';
	for (const b of bytes) out += b.toString(16).padStart(2, '0');
	return out;
}

/**
 * Derive an Ed25519 identity from a BIP39 seed phrase.
 * @param {string} seedPhrase
 * @returns {Promise<Identity>}
 */
export async function deriveIdentity(seedPhrase) {
	const { ed, hmac, sha512, mnemonicToSeedSync } = await libs();
	const seed = mnemonicToSeedSync(seedPhrase.trim(), '');
	const keyMaterial = hmac(sha512, 'ed25519 seed', seed);
	const secretKey = keyMaterial.subarray(0, 32);
	const publicKey = ed.getPublicKey(secretKey);
	return { secretKey, publicKey };
}

/** Public key as lowercase hex (64 chars). */
export function pubkeyHex(identity) {
	return bytesToHex(identity.publicKey);
}

/**
 * Sign an API request. `fullPath` MUST include the `/api/v1` prefix and exclude
 * the query string (the server verifies `format!("/api/v1{}", uri.path())`).
 *
 * @param {Identity} identity
 * @param {string} method   HTTP method (case-insensitive; uppercased for signing)
 * @param {string} fullPath full path incl. /api/v1 prefix, no query string
 * @param {unknown} [bodyData] stringifiable body (objects are JSON-stringified)
 * @returns {{headers: Record<string,string>, body: string}}
 */
export async function signRequest(identity, method, fullPath, bodyData) {
	const { ed } = await libs();
	const nonce = crypto.randomUUID();
	const timestampNs = (BigInt(Date.now()) * 1_000_000n).toString();

	let body = '';
	if (typeof bodyData === 'string') body = bodyData;
	else if (bodyData !== undefined && bodyData !== null) body = JSON.stringify(bodyData);

	const pathNoQuery = fullPath.split('?')[0];
	const message = new TextEncoder().encode(
		timestampNs + nonce + method.toUpperCase() + pathNoQuery + body,
	);
	const signature = ed.sign(message, identity.secretKey, { context: SIGN_CONTEXT });

	return {
		headers: {
			'X-Public-Key': bytesToHex(identity.publicKey),
			'X-Signature': bytesToHex(signature),
			'X-Timestamp': timestampNs,
			'X-Nonce': nonce,
			'Content-Type': 'application/json',
		},
		body,
	};
}
