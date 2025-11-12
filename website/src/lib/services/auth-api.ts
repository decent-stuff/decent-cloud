import { ed25519ph } from '@noble/curves/ed25519';
import type { Ed25519KeyIdentity } from '@dfinity/identity';

const ED25519_SIGN_CONTEXT = new TextEncoder().encode('decent-cloud');

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
	bodyData?: unknown,
	contentType: string = 'application/json'
): Promise<SignedRequest> {
	const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
	const secretKeyBytes = new Uint8Array(identity.getKeyPair().secretKey);

	// Get current timestamp in nanoseconds
	const timestampNs = (Date.now() * 1_000_000).toString();

	// Serialize body
	let body: string;
	if (typeof bodyData === 'string') {
		body = bodyData;
	} else if (bodyData) {
		body = JSON.stringify(bodyData);
	} else {
		body = '';
	}

	// Construct message: timestamp + method + path + body
	const message = new TextEncoder().encode(timestampNs + method + path + body);

	// Sign message using Ed25519ph with SHA-512 prehashing and context (matching DccIdentity)
	const signature = ed25519ph.sign(message, secretKeyBytes.slice(0, 32), { context: ED25519_SIGN_CONTEXT });

	return {
		headers: {
			'X-Public-Key': bytesToHex(publicKeyBytes),
			'X-Signature': bytesToHex(signature),
			'X-Timestamp': timestampNs,
			'Content-Type': contentType
		},
		body
	};
}

function bytesToHex(bytes: Uint8Array): string {
	return Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}
