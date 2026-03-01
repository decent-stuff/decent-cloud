import { ed25519 } from '@noble/curves/ed25519';
import { sha512 } from '@noble/hashes/sha512';
import * as ed from '@noble/ed25519';

ed.hashes.sha512 = sha512;
ed.hashes.sha512Async = (m: Uint8Array) => Promise.resolve(sha512(m));

function encodeString(str: string): Uint8Array {
	const encoder = new TextEncoder();
	const strBytes = encoder.encode(str);
	const lenBytes = new Uint8Array(4);
	new DataView(lenBytes.buffer).setUint32(0, strBytes.length, false);
	return new Uint8Array([...lenBytes, ...strBytes]);
}

function encodeBytes(bytes: Uint8Array): Uint8Array {
	const lenBytes = new Uint8Array(4);
	new DataView(lenBytes.buffer).setUint32(0, bytes.length, false);
	return new Uint8Array([...lenBytes, ...bytes]);
}

function base64Encode(bytes: Uint8Array): string {
	let binary = '';
	for (let i = 0; i < bytes.length; i++) {
		binary += String.fromCharCode(bytes[i]);
	}
	return btoa(binary);
}

function base64Decode(str: string): Uint8Array {
	const binary = atob(str);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) {
		bytes[i] = binary.charCodeAt(i);
	}
	return bytes;
}

export async function generateEd25519KeyPair(): Promise<{
	publicKey: Uint8Array;
	privateKey: Uint8Array;
}> {
	const privateKey = ed25519.utils.randomPrivateKey();
	const publicKey = ed25519.getPublicKey(privateKey);
	return { publicKey, privateKey };
}

export function formatSshPublicKey(publicKey: Uint8Array, comment?: string): string {
	const keyType = 'ssh-ed25519';
	const encoded = new Uint8Array([
		...encodeString(keyType),
		...encodeBytes(publicKey),
	]);
	const base64 = base64Encode(encoded);
	return comment ? `${keyType} ${base64} ${comment}` : `${keyType} ${base64}`;
}

export function formatOpenSshPrivateKey(privateKey: Uint8Array, publicKey: Uint8Array): string {
	const authMagic = new TextEncoder().encode('openssh-key-v1\0');
	const cipherName = encodeString('none');
	const kdfName = encodeString('none');
	const kdfOptions = encodeBytes(new Uint8Array(0));
	const numberOfKeys = new Uint8Array([0, 0, 0, 1]);
	const publicBlob = encodeBytes(new Uint8Array([...encodeString('ssh-ed25519'), ...encodeBytes(publicKey)]));
	const checkInt = new Uint8Array(4);
	crypto.getRandomValues(checkInt);
	const secretKey = new Uint8Array([...privateKey, ...publicKey]);
	const privateKeyData = new Uint8Array([
		...checkInt,
		...checkInt,
		...encodeString('ssh-ed25519'),
		...encodeBytes(publicKey),
		...encodeBytes(secretKey),
		...encodeString('generated-by-decent-cloud'),
	]);
	const paddingLength = 8 - (privateKeyData.length % 8);
	const padding = new Uint8Array(paddingLength);
	for (let i = 0; i < paddingLength; i++) {
		padding[i] = i + 1;
	}
	const paddedPrivateKeyData = new Uint8Array([...privateKeyData, ...padding]);
	const privateBlob = encodeBytes(paddedPrivateKeyData);
	const fullKey = new Uint8Array([
		...authMagic,
		...cipherName,
		...kdfName,
		...kdfOptions,
		...numberOfKeys,
		...publicBlob,
		...privateBlob,
	]);
	const base64 = base64Encode(fullKey);
	const lines = base64.match(/.{1,70}/g) || [];
	return `-----BEGIN OPENSSH PRIVATE KEY-----\n${lines.join('\n')}\n-----END OPENSSH PRIVATE KEY-----\n`;
}

export async function generateSshKeyPair(comment?: string): Promise<{
	publicKeySsh: string;
	privateKeyPem: string;
}> {
	const { publicKey, privateKey } = await generateEd25519KeyPair();
	const publicKeySsh = formatSshPublicKey(publicKey, comment);
	const privateKeyPem = formatOpenSshPrivateKey(privateKey, publicKey);
	return { publicKeySsh, privateKeyPem };
}

export function validateSshPublicKey(key: string): { valid: boolean; error?: string } {
	const trimmed = key.trim();
	if (!trimmed) {
		return { valid: false, error: 'SSH public key is required' };
	}
	const parts = trimmed.split(/\s+/);
	if (parts.length < 2) {
		return { valid: false, error: 'Invalid SSH key format' };
	}
	const [keyType, keyData] = parts;
	if (!['ssh-ed25519', 'ssh-rsa', 'ssh-ecdsa', 'ssh-dss'].includes(keyType)) {
		return { valid: false, error: `Unsupported key type: ${keyType}` };
	}
	try {
		base64Decode(keyData);
		return { valid: true };
	} catch {
		return { valid: false, error: 'Invalid base64 encoding' };
	}
}

export function downloadPrivateKey(privateKeyPem: string, filename: string = 'id_ed25519'): void {
	const blob = new Blob([privateKeyPem], { type: 'application/octet-stream' });
	const url = URL.createObjectURL(blob);
	const a = document.createElement('a');
	a.href = url;
	a.download = filename;
	document.body.appendChild(a);
	a.click();
	document.body.removeChild(a);
	URL.revokeObjectURL(url);
}
