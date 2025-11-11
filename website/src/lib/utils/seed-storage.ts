const SEED_PHRASES_KEY = 'seed_phrases';

export function getStoredSeedPhrases(): string[] {
	if (typeof window === 'undefined') return [];

	try {
		const stored = localStorage.getItem(SEED_PHRASES_KEY);
		if (!stored) return [];
		return JSON.parse(stored);
	} catch (error) {
		console.error('Failed to get stored seed phrases:', error);
		return [];
	}
}

export function setStoredSeedPhrases(phrases: string[]): void {
	if (typeof window === 'undefined') return;

	try {
		localStorage.setItem(SEED_PHRASES_KEY, JSON.stringify(phrases));
	} catch (error) {
		console.error('Failed to set stored seed phrases:', error);
	}
}

export function addSeedPhrase(seedPhrase: string): void {
	const phrases = getStoredSeedPhrases();
	if (!phrases.includes(seedPhrase)) {
		setStoredSeedPhrases([...phrases, seedPhrase]);
	}
}

export function clearStoredSeedPhrases(): void {
	if (typeof window === 'undefined') return;

	try {
		localStorage.removeItem(SEED_PHRASES_KEY);
	} catch (error) {
		console.error('Failed to clear stored seed phrases:', error);
	}
}

export function filterStoredSeedPhrases(filterFn: (phrase: string) => boolean): void {
	const phrases = getStoredSeedPhrases();
	const filtered = phrases.filter(filterFn);
	setStoredSeedPhrases(filtered);
}
