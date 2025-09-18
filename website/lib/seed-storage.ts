const STORAGE_KEY = "seed_phrases";

export interface StorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

function resolveStorage(storage?: StorageLike): StorageLike | null {
  if (storage) {
    return storage;
  }

  if (typeof window === "undefined") {
    return null;
  }

  try {
    return window.localStorage;
  } catch (error) {
    console.warn("Seed storage unavailable:", error);
    return null;
  }
}

function sanitizePhrases(phrases: readonly unknown[]): string[] {
  const unique = new Set<string>();
  const sanitized: string[] = [];

  for (const phrase of phrases) {
    if (typeof phrase !== "string") {
      continue;
    }

    const trimmed = phrase.trim();
    if (!trimmed || unique.has(trimmed)) {
      continue;
    }

    unique.add(trimmed);
    sanitized.push(trimmed);
  }

  return sanitized;
}

function readFromStorage(storage?: StorageLike): string[] {
  const target = resolveStorage(storage);
  if (!target) {
    return [];
  }

  try {
    const raw = target.getItem(STORAGE_KEY);
    if (!raw) {
      return [];
    }

    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return [];
    }

    return sanitizePhrases(parsed);
  } catch (error) {
    console.warn("Failed to read seed phrases:", error);
    return [];
  }
}

function writeToStorage(phrases: readonly string[], storage?: StorageLike): string[] {
  const target = resolveStorage(storage);
  if (!target) {
    return [];
  }

  const sanitized = sanitizePhrases(phrases);

  try {
    target.setItem(STORAGE_KEY, JSON.stringify(sanitized));
  } catch (error) {
    console.warn("Failed to persist seed phrases:", error);
  }

  return sanitized;
}

export function getStoredSeedPhrases(storage?: StorageLike): string[] {
  return readFromStorage(storage);
}

export function setStoredSeedPhrases(
  phrases: readonly string[],
  storage?: StorageLike
): string[] {
  return writeToStorage(phrases, storage);
}

export function addSeedPhrase(phrase: string, storage?: StorageLike): string[] {
  if (!phrase.trim()) {
    return getStoredSeedPhrases(storage);
  }

  const phrases = getStoredSeedPhrases(storage);
  phrases.push(phrase);
  return writeToStorage(phrases, storage);
}

export function removeSeedPhrase(
  phrase: string,
  storage?: StorageLike
): string[] {
  const sanitized = phrase.trim();
  if (!sanitized) {
    return getStoredSeedPhrases(storage);
  }

  return filterStoredSeedPhrases((existing) => existing !== sanitized, storage);
}

export function filterStoredSeedPhrases(
  predicate: (phrase: string) => boolean,
  storage?: StorageLike
): string[] {
  const phrases = getStoredSeedPhrases(storage);
  const filtered = phrases.filter(predicate);
  return writeToStorage(filtered, storage);
}

export function clearStoredSeedPhrases(storage?: StorageLike): void {
  const target = resolveStorage(storage);
  if (!target) {
    return;
  }

  try {
    target.removeItem(STORAGE_KEY);
  } catch (error) {
    console.warn("Failed to clear seed phrases:", error);
  }
}

export function hasSeedPhrases(storage?: StorageLike): boolean {
  return getStoredSeedPhrases(storage).length > 0;
}

export const seedStorageKey = STORAGE_KEY;
