"use client";

import { createContext, useContext, useEffect, useState } from "react";
import { AuthClient } from "@dfinity/auth-client";
import { Identity } from "@dfinity/agent";
import { Principal } from "@dfinity/principal";
import { Ed25519KeyIdentity } from "@dfinity/identity";
import { generateMnemonic, mnemonicToSeedSync } from "bip39";
import { createHmac } from "crypto";

interface IdentityInfo {
  identity: Identity;
  principal: Principal;
  type: "ii" | "nfid" | "seedPhrase";
  name?: string;
}

interface AuthContextType {
  isAuthenticated: boolean;
  currentIdentity: IdentityInfo | null;
  identities: IdentityInfo[];
  loginWithII: (returnUrl?: string) => Promise<void>;
  loginWithNFID: () => Promise<void>;
  loginWithSeedPhrase: (
    seedPhrase?: string,
    returnUrl?: string
  ) => Promise<void>;
  logout: () => Promise<void>;
  switchIdentity: (principal: Principal) => void;
  signOutIdentity: (principal: Principal) => void;
  showSeedPhrase: boolean;
  setShowSeedPhrase: (show: boolean) => void;
}

const AuthContext = createContext<AuthContextType>({
  isAuthenticated: false,
  currentIdentity: null,
  identities: [],
  loginWithII: async () => {},
  loginWithNFID: async () => {},
  loginWithSeedPhrase: async () => {},
  logout: async () => {},
  switchIdentity: () => {},
  signOutIdentity: () => {},
  showSeedPhrase: false,
  setShowSeedPhrase: () => {},
});

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [authClient, setAuthClient] = useState<AuthClient | null>(null);
  const [identities, setIdentities] = useState<IdentityInfo[]>([]);
  const [currentIdentity, setCurrentIdentity] = useState<IdentityInfo | null>(
    null
  );
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [showSeedPhrase, setShowSeedPhrase] = useState(false);

  const signOutIdentity = (principal: Principal) => {
    setIdentities((prev) => {
      const remaining = prev.filter(
        (i) => i.principal.toString() !== principal.toString()
      );
      if (remaining.length === 0) {
        setIsAuthenticated(false);
        setCurrentIdentity(null);
      } else if (
        currentIdentity?.principal.toString() === principal.toString()
      ) {
        // If we're removing the current identity, switch to another one
        setCurrentIdentity(remaining[0]);
      }
      return remaining;
    });

    // Remove from localStorage if it's a seed phrase identity
    const storedSeedPhrases = JSON.parse(
      localStorage.getItem("seed_phrases") || "[]"
    );
    const remainingSeedPhrases = storedSeedPhrases.filter(
      (seedPhrase: string) => {
        const identity = identityFromSeed(seedPhrase);
        return identity.getPrincipal().toString() !== principal.toString();
      }
    );
    localStorage.setItem("seed_phrases", JSON.stringify(remainingSeedPhrases));
  };

  const addIdentity = (identity: Identity, type: IdentityInfo["type"]) => {
    const principal = identity.getPrincipal();
    const newIdentity: IdentityInfo = {
      identity,
      principal,
      type,
    };

    setIdentities((prev) => {
      const existing = prev.find(
        (i) => i.principal.toString() === principal.toString()
      );
      if (existing) {
        // If the identity already exists, update its type if different
        if (existing.type !== type) {
          return prev.map((i) =>
            i.principal.toString() === principal.toString() ? { ...i, type } : i
          );
        }
        return prev;
      }
      return [...prev, newIdentity];
    });

    if (!currentIdentity) {
      setCurrentIdentity(newIdentity);
      setIsAuthenticated(true);
    }
  };

  useEffect(() => {
    // Check if there's a seed phrase in localStorage
    const storedSeedPhrases = JSON.parse(
      localStorage.getItem("seed_phrases") || "[]"
    );

    for (const seedPhrase of storedSeedPhrases) {
      try {
        const identity = identityFromSeed(seedPhrase);
        addIdentity(identity, "seedPhrase");
      } catch (error) {
        console.error("Failed to authenticate with stored seed phrase:", error);
      }
    }

    // Try with AuthClient
    void AuthClient.create().then(async (client) => {
      setAuthClient(client);
      const isAuthenticated = await client.isAuthenticated();
      if (isAuthenticated) {
        const identity = client.getIdentity();
        addIdentity(identity, "ii");
      }
    });
  }, []);

  const loginWithII = async (returnUrl = "/dashboard") => {
    if (!authClient) return;

    // Define session duration (up to 30 days maximum)
    const days = BigInt(1);
    const hours = BigInt(24);
    const nanoseconds = BigInt(3600000000000);
    const maxTimeToLive = days * hours * nanoseconds;

    await authClient.login({
      maxTimeToLive: maxTimeToLive,
      identityProvider: "https://identity.ic0.app",
      onSuccess: () => {
        const identity = authClient.getIdentity();
        addIdentity(identity, "ii");
        window.location.href = returnUrl;
      },
    });
  };

  const loginWithNFID = async () => {
    if (!authClient) return;

    await authClient.login({
      identityProvider: "https://nfid.one",
      onSuccess: () => {
        const identity = authClient.getIdentity();
        addIdentity(identity, "nfid");
        window.location.href = "/dashboard";
      },
    });
  };

  const loginWithSeedPhrase = async (
    existingSeedPhrase?: string,
    returnUrl = "/dashboard"
  ) => {
    try {
      let seedPhrase: string;

      if (existingSeedPhrase) {
        seedPhrase = existingSeedPhrase;
      } else {
        seedPhrase = generateNewSeedPhrase();
      }

      // Store all seed phrases
      const storedSeedPhrases = JSON.parse(
        localStorage.getItem("seed_phrases") || "[]"
      );
      if (!storedSeedPhrases.includes(seedPhrase)) {
        storedSeedPhrases.push(seedPhrase);
        localStorage.setItem("seed_phrases", JSON.stringify(storedSeedPhrases));
      }

      setShowSeedPhrase(true);

      const identity = identityFromSeed(seedPhrase);
      addIdentity(identity, "seedPhrase");
      window.location.href = returnUrl;
    } catch (error) {
      console.error("Failed to login with seed phrase:", error);
      throw error;
    }
  };

  const switchIdentity = (principal: Principal) => {
    const identity = identities.find(
      (i) => i.principal.toString() === principal.toString()
    );
    if (identity) {
      setCurrentIdentity(identity);
      if (identity.type === "ii" || identity.type === "nfid") {
        // When switching to an II or NFID identity, we need to create a new AuthClient
        // with that identity's chain
        void AuthClient.create().then(async (client) => {
          setAuthClient(client);
        });
      }
    }
  };

  const logout = async () => {
    if (authClient) {
      await authClient.logout();
    }

    setIsAuthenticated(false);
    setIdentities([]);
    setCurrentIdentity(null);
    localStorage.removeItem("seed_phrases");
  };

  return (
    <AuthContext.Provider
      value={{
        isAuthenticated,
        currentIdentity,
        identities,
        loginWithII,
        loginWithNFID,
        loginWithSeedPhrase,
        logout,
        switchIdentity,
        signOutIdentity,
        showSeedPhrase,
        setShowSeedPhrase,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function generateNewSeedPhrase(): string {
  return generateMnemonic();
}

export function identityFromSeed(seedPhrase: string): Ed25519KeyIdentity {
  // 1. Generate seed from mnemonic with empty password (matching backend)
  const seed = mnemonicToSeedSync(seedPhrase, "");

  // 2 & 3. Create HMAC-SHA512 with key "ed25519 seed" and feed seed
  const hmac = createHmac("sha512", "ed25519 seed");
  hmac.update(seed);

  // 4. Get first 32 bytes of HMAC output
  const keyMaterial = hmac.digest();
  const seedBytes = keyMaterial.subarray(0, 32);

  // Convert Buffer to ArrayBuffer for DFinity identity
  const privateKeyArrayBuffer = new Uint8Array(seedBytes).buffer;

  // Create DFinity identity from private key
  return Ed25519KeyIdentity.fromSecretKey(privateKeyArrayBuffer);
}

export function useAuth() {
  return useContext(AuthContext);
}
