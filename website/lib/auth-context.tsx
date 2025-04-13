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
  publicKeyBytes?: Uint8Array;
  secretKeyRaw?: Uint8Array;
  seedPhrase?: string;  // Store seed phrase for backup purposes
}

interface AuthenticatedIdentityResult {
  success: true;
  identity: Identity;
  publicKeyBytes: Uint8Array;
  secretKeyRaw: Uint8Array;
}

interface AuthContextType {
  isAuthenticated: boolean;
  currentIdentity: IdentityInfo | null;
  signingIdentity: IdentityInfo | null;
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
  getAuthenticatedIdentity: () => Promise<AuthenticatedIdentityResult | null>;
  getSigningIdentity: () => Promise<AuthenticatedIdentityResult | null>;
  errorMessage: string | null;
  setErrorMessage: (message: string | null) => void;
  backupSeedPhrase: (principal: Principal) => string | null;
  restoreSeedPhrase: (seedPhrase: string) => Promise<void>;
  showBackupInstructions: boolean;
  setShowBackupInstructions: (show: boolean) => void;
}

const AuthContext = createContext<AuthContextType>({
  isAuthenticated: false,
  currentIdentity: null,
  signingIdentity: null,
  identities: [],
  loginWithII: async () => {},
  loginWithNFID: async () => {},
  loginWithSeedPhrase: async () => {},
  logout: async () => {},
  switchIdentity: () => {},
  signOutIdentity: () => {},
  showSeedPhrase: false,
  setShowSeedPhrase: () => {},
  getAuthenticatedIdentity: async () => null,
  getSigningIdentity: async () => null,
  errorMessage: null,
  setErrorMessage: () => {},
  backupSeedPhrase: () => null,
  restoreSeedPhrase: async () => {},
  showBackupInstructions: false,
  setShowBackupInstructions: () => {}
});

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [authClient, setAuthClient] = useState<AuthClient | null>(null);
  const [identities, setIdentities] = useState<IdentityInfo[]>([]);
  const [currentIdentity, setCurrentIdentity] = useState<IdentityInfo | null>(null);
  const [signingIdentity, setSigningIdentity] = useState<IdentityInfo | null>(null);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [showSeedPhrase, setShowSeedPhrase] = useState(false);
  const [showBackupInstructions, setShowBackupInstructions] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const signOutIdentity = (principal: Principal) => {
    setIdentities((prev) => {
      const remaining = prev.filter(
        (i) => i.principal.toString() !== principal.toString()
      );

      // If no identities remain, clear all state
      if (remaining.length === 0) {
        setIsAuthenticated(false);
        setCurrentIdentity(null);
        setSigningIdentity(null);
        return remaining;
      }

      // Find next seed identity if available
      const nextSeedIdentity = remaining.find(i => i.type === "seedPhrase");
      
      // Update current identity if being removed
      if (currentIdentity?.principal.toString() === principal.toString()) {
        // Prefer seed-based identity as current if available
        setCurrentIdentity(nextSeedIdentity || remaining[0]);
      }

      // Update signing identity if being removed or none exists
      if (!signingIdentity || signingIdentity.principal.toString() === principal.toString()) {
        if (!nextSeedIdentity) {
          // Auto-generate new seed identity if none exists
          try {
            const seedPhrase = generateNewSeedPhrase();
            const identity = identityFromSeed(seedPhrase);
            const keyPair = identity.getKeyPair();
            const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
            const secretKeyRaw = new Uint8Array(keyPair.secretKey);

            // Add to remaining identities
            const newIdentity = {
              identity,
              principal: identity.getPrincipal(),
              type: "seedPhrase" as const,
              publicKeyBytes,
              secretKeyRaw,
              seedPhrase
            };
            remaining.push(newIdentity);
            
            // Store seed phrase
            const storedSeedPhrases = JSON.parse(localStorage.getItem("seed_phrases") || "[]");
            storedSeedPhrases.push(seedPhrase);
            localStorage.setItem("seed_phrases", JSON.stringify(storedSeedPhrases));
            
            // Set as signing identity and show backup dialog
            setSigningIdentity(newIdentity);
            setShowSeedPhrase(true);
            setShowBackupInstructions(true);
          } catch (error) {
            console.error("Failed to create new seed identity:", error);
            // Continue with remaining identities - user will be prompted to add seed identity later
          }
        } else {
          setSigningIdentity(nextSeedIdentity);
        }
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

  const addIdentity = (
    identity: Identity,
    type: IdentityInfo["type"],
    publicKeyBytes?: Uint8Array,
    secretKeyRaw?: Uint8Array,
    seedPhrase?: string
  ) => {
    const principal = identity.getPrincipal();
    const newIdentity: IdentityInfo = {
      identity,
      principal,
      type,
      publicKeyBytes,
      secretKeyRaw,
      seedPhrase
    };

    setIdentities((prev) => {
      // For seed phrase identities, check if this exact seed phrase already exists
      if (type === "seedPhrase" && seedPhrase) {
        const hasExactPhrase = prev.some(i => i.seedPhrase === seedPhrase);
        if (hasExactPhrase) {
          return prev; // Skip if exact seed phrase exists
        }
      } else {
        // For non-seed identities, update existing identity if found
        const existing = prev.find(
          (i) => i.principal.toString() === principal.toString()
        );
        if (existing) {
        // If the identity already exists, update its type and keys if any part is different
          if (existing.type !== type || !existing.publicKeyBytes || !existing.secretKeyRaw) {
            return prev.map((i) =>
              i.principal.toString() === principal.toString()
                ? { ...i, type, publicKeyBytes, secretKeyRaw }
                : i
            );
          }
          return prev;
        }
      }
      return [...prev, newIdentity];
    });

    if (!currentIdentity) {
      setCurrentIdentity(newIdentity);
      setIsAuthenticated(true);
    }
    
    // Only handle seed phrase identities if this is a new non-seed identity
    if (type !== "seedPhrase" && !signingIdentity) {
      // Check for existing seed identities in both current state and localStorage
      const hasSeedIdentity = identities.some(i => i.type === "seedPhrase");
      const storedSeedPhrases = JSON.parse(localStorage.getItem("seed_phrases") || "[]");
      const hasStoredSeedPhrases = storedSeedPhrases.length > 0;
      
      if (!hasSeedIdentity && !hasStoredSeedPhrases) {
        // Only create a new seed identity if none exist anywhere
        try {
          const seedPhrase = generateNewSeedPhrase();
          const identity = identityFromSeed(seedPhrase);
          const keyPair = identity.getKeyPair();
          const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
          const secretKeyRaw = new Uint8Array(keyPair.secretKey);

          const seedIdentity = {
            identity,
            principal: identity.getPrincipal(),
            type: "seedPhrase" as const,
            publicKeyBytes,
            secretKeyRaw,
            seedPhrase
          };

          // Store seed phrase
          const storedSeedPhrases = JSON.parse(localStorage.getItem("seed_phrases") || "[]");
          storedSeedPhrases.push(seedPhrase);
          localStorage.setItem("seed_phrases", JSON.stringify(storedSeedPhrases));
          
          // Mark as new seed phrase to trigger backup dialog
          localStorage.setItem("new_seed_phrase", "true");

          // Add to identities and set as signing identity
          setIdentities(prev => [...prev, seedIdentity]);
          setSigningIdentity(seedIdentity);
          setShowSeedPhrase(true);
          setShowBackupInstructions(true);
        } catch (error) {
          console.error("Failed to create seed identity:", error);
          setErrorMessage("A seed-based identity is required for signing updates. Failed to create one automatically.");
        }
      }
    } else if (type === "seedPhrase") {
      // For existing seed identities, set as signing identity
      setSigningIdentity(newIdentity);
    }
  };

  const getAuthenticatedIdentity = async (): Promise<AuthenticatedIdentityResult | null> => {
    if (!currentIdentity ||
        currentIdentity.type !== "seedPhrase" ||
        !currentIdentity.publicKeyBytes ||
        !currentIdentity.secretKeyRaw) {
      return null;
    }

    const { identity, publicKeyBytes, secretKeyRaw } = currentIdentity;
    return {
      success: true,
      identity,
      publicKeyBytes,
      secretKeyRaw
    };
  };

  const getSigningIdentity = async (): Promise<AuthenticatedIdentityResult | null> => {
    if (!signingIdentity ||
        !signingIdentity.publicKeyBytes ||
        !signingIdentity.secretKeyRaw) {
      return null;
    }

    const { identity, publicKeyBytes, secretKeyRaw } = signingIdentity;
    return {
      success: true,
      identity,
      publicKeyBytes,
      secretKeyRaw
    };
  };

  useEffect(() => {
    const initializeAuth = async () => {
      // Check for and migrate old format seed phrase if it exists
      const oldSeedPhrase = localStorage.getItem("seed_phrase");
      if (oldSeedPhrase) {
        const storedSeedPhrases = JSON.parse(localStorage.getItem("seed_phrases") || "[]");
        if (!storedSeedPhrases.includes(oldSeedPhrase)) {
          storedSeedPhrases.push(oldSeedPhrase);
          localStorage.setItem("seed_phrases", JSON.stringify(storedSeedPhrases));
        }
        // Clean up old format
        localStorage.removeItem("seed_phrase");
        localStorage.removeItem("identity_key");
      }

      // Get current seed phrases
      const storedSeedPhrases = JSON.parse(
        localStorage.getItem("seed_phrases") || "[]"
      );

      const validPhrases: string[] = [];
      let foundSigningIdentity = false;

      // Restore seed phrase identities first
      for (const seedPhrase of storedSeedPhrases) {
        try {
          const identity = identityFromSeed(seedPhrase);
          const keyPair = identity.getKeyPair();
          const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
          const secretKeyRaw = new Uint8Array(keyPair.secretKey);
          
          validPhrases.push(seedPhrase);
          
          // Add identity but don't auto-create new seed identity
          addIdentity(identity, "seedPhrase", publicKeyBytes, secretKeyRaw, seedPhrase);
          
          if (!foundSigningIdentity) {
            setSigningIdentity({
              identity,
              principal: identity.getPrincipal(),
              type: "seedPhrase",
              publicKeyBytes,
              secretKeyRaw,
              seedPhrase
            });
            foundSigningIdentity = true;
          }
        } catch (error) {
          console.error("Failed to authenticate with stored seed phrase:", error);
        }
      }

      // Update localStorage to remove any invalid phrases
      if (validPhrases.length !== storedSeedPhrases.length) {
        localStorage.setItem("seed_phrases", JSON.stringify(validPhrases));
      }

      // Then try with AuthClient
      try {
        const client = await AuthClient.create();
        setAuthClient(client);
        const isAuthenticated = await client.isAuthenticated();
        if (isAuthenticated) {
          const identity = client.getIdentity();
          addIdentity(identity, "ii");
        }
      } catch (error) {
        console.error("Failed to initialize AuthClient:", error);
      }
    };

    void initializeAuth();
  }, []);

  const loginWithII = async (returnUrl = "/dashboard") => {
    if (!authClient) return;

    // Define session duration: 1 day (it can be up to 30 days max)
    const days = 1;
    const maxTimeToLive = BigInt(days) * BigInt(24) * BigInt(3600000000000);

    await authClient.login({
      maxTimeToLive: maxTimeToLive,
      identityProvider: "https://identity.ic0.app",
      onSuccess: async () => {
        const identity = authClient.getIdentity();
        addIdentity(identity, "ii");

        // Let existing auto-generation logic in addIdentity handle seed identity if needed
        window.location.href = returnUrl;
      },
    });
  };

  const loginWithNFID = async () => {
    if (!authClient) return;

    await authClient.login({
      identityProvider: "https://nfid.one",
      onSuccess: async () => {
        const identity = authClient.getIdentity();
        addIdentity(identity, "nfid");
        
        // Let existing auto-generation logic in addIdentity handle seed identity if needed
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
      const keyPair = identity.getKeyPair();
      addIdentity(
        identity,
        "seedPhrase",
        new Uint8Array(identity.getPublicKey().rawKey),
        new Uint8Array(keyPair.secretKey),
        seedPhrase
      );

      if (!existingSeedPhrase) {
        setShowBackupInstructions(true);
        setShowSeedPhrase(true);
      }

      window.location.href = returnUrl;
    } catch (error) {
      console.error("Failed to login with seed phrase:", error);
      throw error;
    }
  };

  const backupSeedPhrase = (principal: Principal): string | null => {
    const identity = identities.find(i => i.principal.toString() === principal.toString());
    
    if (!identity) {
      setErrorMessage("Identity not found");
      return null;
    }

    if (identity.type !== "seedPhrase") {
      setErrorMessage("This identity does not use a recovery phrase");
      return null;
    }

    // Return stored seed phrase if available
    if (identity.seedPhrase) {
      return identity.seedPhrase;
    }

    // Try to find matching seed phrase in storage
    const storedPhrases = JSON.parse(localStorage.getItem("seed_phrases") || "[]") as string[];
    const matchingPhrase = storedPhrases.find((phrase: string) => {
      try {
        const testIdentity = identityFromSeed(phrase);
        return testIdentity.getPrincipal().toString() === principal.toString();
      } catch {
        return false;
      }
    });

    if (matchingPhrase) {
      identity.seedPhrase = matchingPhrase; // Update identity with found phrase
      return matchingPhrase;
    }

    setErrorMessage("Recovery phrase not found - possible data loss");
    return null;
  };

  const restoreSeedPhrase = async (seedPhrase: string): Promise<void> => {
    try {
      // Validate and login with seed phrase
      await loginWithSeedPhrase(seedPhrase);
    } catch (error) {
      console.error("Failed to restore seed phrase:", error);
      throw error;
    }
  };

  const switchIdentity = (principal: Principal) => {
    const targetIdentity = identities.find(
      (i) => i.principal.toString() === principal.toString()
    );
    if (!targetIdentity) return;

    // Update current identity
    setCurrentIdentity(targetIdentity);

    // Handle signing identity updates
    if (targetIdentity.type === "seedPhrase") {
      setSigningIdentity(targetIdentity);
    } else {
      // For non-seed identities, keep or find a seed-based signing identity
      if (!signingIdentity || signingIdentity.type !== "seedPhrase") {
        const seedIdentity = identities.find(i => i.type === "seedPhrase");
        if (seedIdentity) {
          setSigningIdentity(seedIdentity);
        }
      }
    }

    // Update auth client for II/NFID identities
    if (targetIdentity.type === "ii" || targetIdentity.type === "nfid") {
      void AuthClient.create().then(setAuthClient);
    }
  };

  const logout = async () => {
    if (authClient) {
      await authClient.logout();
    }

    // Complete clearing of all auth state
    setIsAuthenticated(false);
    setIdentities([]);
    setCurrentIdentity(null);
    setSigningIdentity(null);
    setShowSeedPhrase(false);
    setShowBackupInstructions(false);
    setErrorMessage(null);
    localStorage.removeItem("seed_phrases");
  };

  return (
    <AuthContext.Provider
      value={{
        isAuthenticated,
        currentIdentity,
        signingIdentity,
        identities,
        loginWithII,
        loginWithNFID,
        loginWithSeedPhrase,
        logout,
        switchIdentity,
        signOutIdentity,
        showSeedPhrase,
        setShowSeedPhrase,
        getAuthenticatedIdentity,
        getSigningIdentity,
        errorMessage,
        setErrorMessage,
        backupSeedPhrase,
        restoreSeedPhrase,
        showBackupInstructions,
        setShowBackupInstructions
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

export { type AuthenticatedIdentityResult };
export { type IdentityInfo };
