import { sha512 } from '@noble/hashes/sha512';
import * as ed from '@noble/ed25519';
import type { Ed25519KeyIdentity } from '@dfinity/identity';

// Set the sha512 hash for ed25519 v3
ed.hashes.sha512 = sha512;
ed.hashes.sha512Async = (m: Uint8Array) => Promise.resolve(sha512(m));

export interface SignedRequest {
	headers: {
		'X-Public-Key': string;
		'X-Signature': string;
		'X-Timestamp': string;
		'Content-Type': string;
	};
	body: string;
}

/**
 * Sign an API request with Ed25519 key
 * Message format: timestamp + method + path + body
 */
export async function signRequest(
	identity: Ed25519KeyIdentity,
	method: string,
	path: string,
	bodyData?: unknown
): Promise<SignedRequest> {
	const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
	const secretKeyBytes = new Uint8Array(identity.getKeyPair().secretKey);

	// Get current timestamp in nanoseconds
	const timestampNs = (Date.now() * 1_000_000).toString();

	// Serialize body
	const body = bodyData ? JSON.stringify(bodyData) : '';

	// Construct message: timestamp + method + path + body
	const message = new TextEncoder().encode(timestampNs + method + path + body);

	// Sign message (Ed25519 with SHA-512 prehashing)
	const prehashed = sha512(message);

	// Sign with Ed25519
	const signature = await ed.sign(prehashed, secretKeyBytes.slice(0, 32));

	return {
		headers: {
			'X-Public-Key': bytesToHex(publicKeyBytes),
			'X-Signature': bytesToHex(signature),
			'X-Timestamp': timestampNs,
			'Content-Type': 'application/json'
		},
		body
	};
}

function bytesToHex(bytes: Uint8Array): string {
	return Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}
