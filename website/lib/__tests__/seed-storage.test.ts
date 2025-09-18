import {
  addSeedPhrase,
  clearStoredSeedPhrases,
  filterStoredSeedPhrases,
  getStoredSeedPhrases,
  hasSeedPhrases,
  removeSeedPhrase,
  setStoredSeedPhrases,
  type StorageLike,
} from "@/lib/seed-storage";

describe("seed-storage", () => {
  class MemoryStorage implements StorageLike {
    private store = new Map<string, string>();

    getItem(key: string): string | null {
      return this.store.has(key) ? (this.store.get(key) as string) : null;
    }

    setItem(key: string, value: string): void {
      this.store.set(key, value);
    }

    removeItem(key: string): void {
      this.store.delete(key);
    }
  }

  const createStorage = () => new MemoryStorage();

  it("adds seed phrases exactly once and trims whitespace", () => {
    const storage = createStorage();

    addSeedPhrase("  alpha beta gamma  ", storage);
    addSeedPhrase("alpha beta gamma", storage);

    expect(getStoredSeedPhrases(storage)).toEqual(["alpha beta gamma"]);
  });

  it("removes phrases and reports presence correctly", () => {
    const storage = createStorage();

    addSeedPhrase("alpha", storage);
    addSeedPhrase("beta", storage);

    expect(hasSeedPhrases(storage)).toBe(true);

    removeSeedPhrase("alpha", storage);

    expect(getStoredSeedPhrases(storage)).toEqual(["beta"]);

    removeSeedPhrase("beta", storage);

    expect(getStoredSeedPhrases(storage)).toEqual([]);
    expect(hasSeedPhrases(storage)).toBe(false);
  });

  it("filters stored phrases with predicate", () => {
    const storage = createStorage();

    setStoredSeedPhrases(["alpha", "beta", "gamma"], storage);

    const filtered = filterStoredSeedPhrases(
      (phrase) => phrase !== "beta",
      storage
    );

    expect(filtered).toEqual(["alpha", "gamma"]);
    expect(getStoredSeedPhrases(storage)).toEqual(["alpha", "gamma"]);
  });

  it("sanitizes invalid stored values", () => {
    const storage = createStorage();

    storage.setItem(
      "seed_phrases",
      JSON.stringify(["alpha", 123, "", "beta", "alpha"])
    );

    expect(getStoredSeedPhrases(storage)).toEqual(["alpha", "beta"]);
  });

  it("clears the storage", () => {
    const storage = createStorage();

    addSeedPhrase("alpha", storage);
    clearStoredSeedPhrases(storage);

    expect(getStoredSeedPhrases(storage)).toEqual([]);
  });
});
