import { describe, it, expect, beforeEach } from 'vitest';
import {
	getStoredSeedPhrases,
	setStoredSeedPhrases,
	addSeedPhrase,
	clearStoredSeedPhrases
} from './seed-storage';

describe('seed-storage', () => {
	beforeEach(() => {
		clearStoredSeedPhrases();
	});

	it('should store and retrieve seed phrases', () => {
		const seedPhrase = 'test seed phrase with twelve words to make it valid here now';

		addSeedPhrase(seedPhrase);
		const retrieved = getStoredSeedPhrases();

		expect(retrieved).toContain(seedPhrase);
		expect(retrieved.length).toBe(1);
	});

	it('should return empty array when no seed phrases saved', () => {
		const retrieved = getStoredSeedPhrases();

		expect(retrieved).toEqual([]);
	});

	it('should handle multiple seed phrases', () => {
		const seed1 = 'first seed phrase with twelve words to make it valid here now';
		const seed2 = 'second seed phrase with twelve words to make it valid here now';

		addSeedPhrase(seed1);
		addSeedPhrase(seed2);

		const retrieved = getStoredSeedPhrases();

		expect(retrieved).toContain(seed1);
		expect(retrieved).toContain(seed2);
		expect(retrieved.length).toBe(2);
	});

	it('should not add duplicate seed phrases', () => {
		const seedPhrase = 'test seed phrase with twelve words to make it valid here now';

		addSeedPhrase(seedPhrase);
		addSeedPhrase(seedPhrase);

		const retrieved = getStoredSeedPhrases();

		expect(retrieved.length).toBe(1);
	});

	it('should clear all seed phrases', () => {
		const seed1 = 'first seed phrase with twelve words to make it valid here now';
		const seed2 = 'second seed phrase with twelve words to make it valid here now';

		addSeedPhrase(seed1);
		addSeedPhrase(seed2);
		expect(getStoredSeedPhrases().length).toBe(2);

		clearStoredSeedPhrases();

		expect(getStoredSeedPhrases()).toEqual([]);
	});

	it('should set seed phrases array directly', () => {
		const phrases = [
			'first seed phrase with twelve words to make it valid here now',
			'second seed phrase with twelve words to make it valid here now'
		];

		setStoredSeedPhrases(phrases);

		expect(getStoredSeedPhrases()).toEqual(phrases);
	});
});
