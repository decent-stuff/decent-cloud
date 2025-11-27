import { describe, it, expect } from 'vitest';
import { validateUsernameFormat } from './account-api';

describe('validateUsernameFormat', () => {
	describe('valid usernames', () => {
		it('accepts lowercase alphanumeric', () => {
			expect(validateUsernameFormat('alice')).toBeNull();
			expect(validateUsernameFormat('bob123')).toBeNull();
		});

		it('accepts uppercase (preserves case)', () => {
			expect(validateUsernameFormat('ALICE')).toBeNull();
			expect(validateUsernameFormat('Bob123')).toBeNull();
			expect(validateUsernameFormat('MixedCase')).toBeNull();
		});

		it('accepts special characters in middle', () => {
			expect(validateUsernameFormat('alice.smith')).toBeNull();
			expect(validateUsernameFormat('user_99')).toBeNull();
			expect(validateUsernameFormat('charlie-delta')).toBeNull();
			expect(validateUsernameFormat('user@example.com')).toBeNull();
		});

		it('trims whitespace', () => {
			expect(validateUsernameFormat('  alice  ')).toBeNull();
			expect(validateUsernameFormat('\talice\t')).toBeNull();
		});
	});

	describe('length validation', () => {
		it('rejects usernames shorter than 3 characters', () => {
			expect(validateUsernameFormat('ab')).toBe('Username too short (minimum 3 characters)');
			expect(validateUsernameFormat('a')).toBe('Username too short (minimum 3 characters)');
			expect(validateUsernameFormat('')).toBe('Username too short (minimum 3 characters)');
		});

		it('rejects usernames longer than 64 characters', () => {
			const longUsername = 'a'.repeat(65);
			expect(validateUsernameFormat(longUsername)).toBe(
				'Username too long (maximum 64 characters)'
			);
		});

		it('accepts username at minimum length', () => {
			expect(validateUsernameFormat('abc')).toBeNull();
		});

		it('accepts username at maximum length', () => {
			const maxUsername = 'a'.repeat(64);
			expect(validateUsernameFormat(maxUsername)).toBeNull();
		});
	});

	describe('format validation', () => {
		it('rejects username starting with special character', () => {
			expect(validateUsernameFormat('-alice')).toBe(
				'Username must start with a letter or number'
			);
			expect(validateUsernameFormat('.alice')).toBe(
				'Username must start with a letter or number'
			);
			expect(validateUsernameFormat('_alice')).toBe(
				'Username must start with a letter or number'
			);
			expect(validateUsernameFormat('@alice')).toBe(
				'Username must start with a letter or number'
			);
		});

		it('rejects username ending with special character', () => {
			expect(validateUsernameFormat('alice-')).toBe(
				'Username must end with a letter or number'
			);
			expect(validateUsernameFormat('alice.')).toBe(
				'Username must end with a letter or number'
			);
			expect(validateUsernameFormat('alice_')).toBe(
				'Username must end with a letter or number'
			);
			expect(validateUsernameFormat('alice@')).toBe(
				'Username must end with a letter or number'
			);
		});

		it('reports specific invalid characters', () => {
			const result = validateUsernameFormat('alice!bob');
			expect(result).toContain('Invalid character(s): !');

			const result2 = validateUsernameFormat('alice bob');
			expect(result2).toContain('Invalid character(s):  ');

			const result3 = validateUsernameFormat('alice#bob$test');
			expect(result3).toContain('Invalid character(s)');
			expect(result3).toContain('#');
			expect(result3).toContain('$');
		});
	});

	describe('reserved usernames', () => {
		const reserved = [
			'admin',
			'api',
			'system',
			'root',
			'support',
			'moderator',
			'administrator',
			'test',
			'null',
			'undefined',
			'decent',
			'cloud'
		];

		it.each(reserved)('rejects reserved username: %s', (username) => {
			expect(validateUsernameFormat(username)).toBe('This username is reserved');
		});

		it('rejects reserved usernames case-insensitively', () => {
			expect(validateUsernameFormat('ADMIN')).toBe('This username is reserved');
			expect(validateUsernameFormat('Admin')).toBe('This username is reserved');
			expect(validateUsernameFormat('SyStEm')).toBe('This username is reserved');
		});
	});
});
