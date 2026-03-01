import { describe, it, expect } from 'vitest';
import { execSync } from 'child_process';
import { writeFileSync, unlinkSync, existsSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import {
	generateEd25519KeyPair,
	formatSshPublicKey,
	formatOpenSshPrivateKey,
	generateSshKeyPair,
	validateSshPublicKey,
} from './ssh-keygen';

describe('ssh-keygen', () => {
	describe('generateEd25519KeyPair', () => {
		it('should generate a valid Ed25519 keypair', async () => {
			const { publicKey, privateKey } = await generateEd25519KeyPair();
			expect(publicKey).toBeInstanceOf(Uint8Array);
			expect(privateKey).toBeInstanceOf(Uint8Array);
			expect(publicKey.length).toBe(32);
			expect(privateKey.length).toBe(32);
		});

		it('should generate different keypairs each time', async () => {
			const kp1 = await generateEd25519KeyPair();
			const kp2 = await generateEd25519KeyPair();
			expect(kp1.publicKey).not.toEqual(kp2.publicKey);
			expect(kp1.privateKey).not.toEqual(kp2.privateKey);
		});
	});

	describe('formatSshPublicKey', () => {
		it('should format public key in OpenSSH format', async () => {
			const { publicKey } = await generateEd25519KeyPair();
			const formatted = formatSshPublicKey(publicKey);
			expect(formatted).toMatch(/^ssh-ed25519 [A-Za-z0-9+/]+=*$/);
		});

		it('should include comment when provided', async () => {
			const { publicKey } = await generateEd25519KeyPair();
			const formatted = formatSshPublicKey(publicKey, 'user@example.com');
			expect(formatted).toMatch(/^ssh-ed25519 [A-Za-z0-9+/]+=* user@example\.com$/);
		});

		it('should produce valid base64 that decodes to correct structure', async () => {
			const { publicKey } = await generateEd25519KeyPair();
			const formatted = formatSshPublicKey(publicKey);
			const parts = formatted.split(' ');
			expect(parts.length).toBe(2);
			expect(parts[0]).toBe('ssh-ed25519');
			const base64 = parts[1];
			const binary = atob(base64);
			const bytes = new Uint8Array(binary.length);
			for (let i = 0; i < binary.length; i++) {
				bytes[i] = binary.charCodeAt(i);
			}
			const view = new DataView(bytes.buffer);
			const keyTypeLen = view.getUint32(0, false);
			expect(keyTypeLen).toBe(11);
			const keyType = new TextDecoder().decode(bytes.slice(4, 4 + keyTypeLen));
			expect(keyType).toBe('ssh-ed25519');
			const pubKeyLen = view.getUint32(15, false);
			expect(pubKeyLen).toBe(32);
		});
	});

	describe('formatOpenSshPrivateKey', () => {
		it('should format private key in OpenSSH PEM format', async () => {
			const { publicKey, privateKey } = await generateEd25519KeyPair();
			const formatted = formatOpenSshPrivateKey(privateKey, publicKey);
			expect(formatted).toContain('-----BEGIN OPENSSH PRIVATE KEY-----');
			expect(formatted).toContain('-----END OPENSSH PRIVATE KEY-----');
		});

		it('should start with openssh-key-v1 magic', async () => {
			const { publicKey, privateKey } = await generateEd25519KeyPair();
			const formatted = formatOpenSshPrivateKey(privateKey, publicKey);
			const base64Part = formatted
				.replace('-----BEGIN OPENSSH PRIVATE KEY-----\n', '')
				.replace('\n-----END OPENSSH PRIVATE KEY-----\n', '')
				.replace(/\n/g, '');
			const binary = atob(base64Part);
			expect(binary.startsWith('openssh-key-v1\0')).toBe(true);
		});
	});

	describe('generateSshKeyPair', () => {
		it('should generate complete SSH keypair', async () => {
			const { publicKeySsh, privateKeyPem } = await generateSshKeyPair();
			expect(publicKeySsh).toMatch(/^ssh-ed25519 [A-Za-z0-9+/]+=*$/);
			expect(privateKeyPem).toContain('-----BEGIN OPENSSH PRIVATE KEY-----');
			expect(privateKeyPem).toContain('-----END OPENSSH PRIVATE KEY-----');
		});

		it('should include comment in public key when provided', async () => {
			const { publicKeySsh } = await generateSshKeyPair('test@decent-cloud.org');
			expect(publicKeySsh).toContain('test@decent-cloud.org');
		});
	});

	describe('validateSshPublicKey', () => {
		it('should validate a generated ssh-ed25519 key', async () => {
			const { publicKeySsh } = await generateSshKeyPair();
			const result = validateSshPublicKey(publicKeySsh);
			expect(result.valid).toBe(true);
		});

		it('should reject empty key', () => {
			const result = validateSshPublicKey('');
			expect(result.valid).toBe(false);
			expect(result.error).toContain('required');
		});

		it('should reject key without data', () => {
			const result = validateSshPublicKey('ssh-ed25519');
			expect(result.valid).toBe(false);
		});

		it('should reject unsupported key type', () => {
			const result = validateSshPublicKey('ssh-unknown AAAAB3NzaC1yc2E=');
			expect(result.valid).toBe(false);
			expect(result.error).toContain('Unsupported key type');
		});

		it('should accept ssh-rsa key format', () => {
			const rsaKey = 'ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQC7 test@example.com';
			const result = validateSshPublicKey(rsaKey);
			expect(result.valid).toBe(true);
		});
	});

	describe('OpenSSH compatibility', () => {
		it('should generate a key readable by ssh-keygen', async () => {
			const { publicKeySsh, privateKeyPem } = await generateSshKeyPair('test@decent-cloud.org');
			const keyPath = join(tmpdir(), `test-ssh-key-${Date.now()}`);
			try {
				writeFileSync(keyPath, privateKeyPem, { mode: 0o600 });
				const extractedPubKey = execSync(`ssh-keygen -y -f "${keyPath}"`, {
					encoding: 'utf-8'
				}).trim();
				const [generatedType, generatedData] = publicKeySsh.split(' ');
				const [extractedType, extractedData] = extractedPubKey.split(' ');
				expect(extractedType).toBe(generatedType);
				expect(extractedData).toBe(generatedData);
			} finally {
				if (existsSync(keyPath)) {
					unlinkSync(keyPath);
				}
			}
		});
	});
});
