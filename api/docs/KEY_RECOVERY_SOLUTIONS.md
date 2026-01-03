# Secure Key Recovery for Internet Identity Users

## Problem Statement

When users authenticate with Internet Identity (II), they get a stable principal ID but no signing key. Your app auto-generates a seed phrase for signing, but if the user loses their device, they lose access to their signing key and all associated data/assets.

**Goal**: Enable users to securely recover their signing key when re-authenticating with II (e.g., Google, Face ID) on a new device.

## Solution Options

### Option 1: VetKeys (ICP Native) ⭐ RECOMMENDED

**What are VetKeys?**
- Launched July 2025 as part of ICP's Niobium upgrade
- Verifiably encrypted threshold key derivation (vetKD) protocol
- Keys derived by subnet nodes, no single node sees the key
- Deterministic: Same identity → Same derived key
- Identity-Based Encryption (IBE): Encrypt directly to II principal

**How it works for key recovery:**

```rust
// Canister-side (Motoko/Rust)
// 1. User logs in with II for the first time
let user_principal = caller();  // e.g., "abc123-..."

// 2. Derive a deterministic encryption key for this user's II principal
let transport_key = vetkd_derive_key(
    derivation_path: format!("user:{}", user_principal),
    user_public_key: user_session_key,  // From II delegation
);

// 3. User's device generates signing key (seed phrase)
// Frontend sends encrypted seed to canister
let encrypted_seed = encrypt_with_transport_key(seed_phrase, transport_key);

// 4. Store encrypted seed in canister
user_encrypted_seeds.insert(user_principal, encrypted_seed);

// 5. Later: User re-authenticates on new device
// Canister derives SAME transport key (deterministic!)
let recovered_transport_key = vetkd_derive_key(
    derivation_path: format!("user:{}", user_principal),
    user_public_key: new_session_key,
);

// 6. Decrypt and return to new device
let decrypted_seed = decrypt_with_transport_key(encrypted_seed, recovered_transport_key);
```

**Frontend flow:**

```typescript
// Initial setup
async function setupKeyRecovery(authClient: AuthClient) {
  const identity = authClient.getIdentity();
  const principal = identity.getPrincipal();

  // Generate seed phrase
  const seedPhrase = generateMnemonic();
  const signingKey = deriveSeedPhrase(seedPhrase);

  // Get transport key from canister (via vetKD)
  const transportKey = await canister.derive_user_transport_key(principal);

  // Encrypt seed phrase with transport key
  const encryptedSeed = await encryptWithTransportKey(seedPhrase, transportKey);

  // Upload encrypted seed to canister
  await canister.store_encrypted_seed(principal, encryptedSeed);

  // User still needs to backup seed phrase as fallback
  return seedPhrase;
}

// Key recovery on new device
async function recoverKey(authClient: AuthClient) {
  const identity = authClient.getIdentity();
  const principal = identity.getPrincipal();

  // Get NEW transport key (same derivation path → same key!)
  const transportKey = await canister.derive_user_transport_key(principal);

  // Fetch encrypted seed from canister
  const encryptedSeed = await canister.get_encrypted_seed(principal);

  // Decrypt with transport key
  const seedPhrase = await decryptWithTransportKey(encryptedSeed, transportKey);

  // Restore signing identity
  return deriveSeedPhrase(seedPhrase);
}
```

**Pros:**
- ✅ Fully decentralized (no central server)
- ✅ No password required (uses II identity)
- ✅ Deterministic recovery (same II → same encryption key)
- ✅ Native ICP integration
- ✅ Encrypted keys never leave the IC in plaintext
- ✅ Works across all II authentication methods (Google, WebAuthn, etc.)

**Cons:**
- ⚠️ Requires canister implementation (additional complexity)
- ⚠️ Cycles cost for key derivation operations
- ⚠️ Still beta (launched July 2025, may have rough edges)
- ⚠️ Users lose access if they lose II credentials (no fallback)

**Resources:**
- [VetKeys Documentation](https://internetcomputer.org/docs/building-apps/network-features/vetkeys/introduction)
- [GitHub Examples](https://github.com/dfinity/vetkeys)
- Rust crate: `ic-vetkeys`
- Motoko library: `ic-vetkeys` (mops.one)

---

### Option 2: OPRF-Based Key Derivation (Password-Augmented)

**What is OPRF?**
- Oblivious Pseudorandom Function
- Server helps derive key without learning the password
- Used by WhatsApp/Signal for encrypted backups
- OPAQUE protocol (IETF standard)

**How it works:**

```rust
// Backend (API server)
struct OPRFServer {
    // Server has a secret key, never revealed
    server_secret: [u8; 32],
}

// 1. User sets recovery password
impl OPRFServer {
    fn blind_evaluate(&self, blinded_password: &[u8]) -> [u8; 32] {
        // Server computes PRF without seeing password
        oprf_evaluate(self.server_secret, blinded_password)
    }
}

// 2. Client derives encryption key from password + server response
async fn derive_recovery_key(password: &str, server: &OPRFServer) -> [u8; 32] {
    // Client blinds password
    let (blinded, unblinding_factor) = oprf_blind(password);

    // Server evaluates (doesn't see password!)
    let server_output = server.blind_evaluate(&blinded);

    // Client unblinds to get final key
    let recovery_key = oprf_unblind(server_output, unblinding_factor);

    recovery_key  // Use this to encrypt seed phrase
}
```

**Frontend flow:**

```typescript
// Setup recovery
async function setupPasswordRecovery(seedPhrase: string) {
  // User sets recovery password
  const password = prompt("Set recovery password:");

  // Derive encryption key via OPRF
  const recoveryKey = await deriveRecoveryKey(password);

  // Encrypt seed phrase
  const encryptedSeed = await encrypt(seedPhrase, recoveryKey);

  // Store encrypted seed on backend
  await api.post('/users/encrypted-seed', { encryptedSeed });
}

// Recovery
async function recoverWithPassword(principal: string) {
  // User enters recovery password
  const password = prompt("Enter recovery password:");

  // Derive SAME encryption key via OPRF
  const recoveryKey = await deriveRecoveryKey(password);

  // Fetch encrypted seed
  const { encryptedSeed } = await api.get(`/users/${principal}/encrypted-seed`);

  // Decrypt
  const seedPhrase = await decrypt(encryptedSeed, recoveryKey);

  return seedPhrase;
}
```

**Pros:**
- ✅ Works with any backend (not ICP-specific)
- ✅ Server never sees password or seed
- ✅ IETF standard (OPAQUE)
- ✅ Battle-tested (WhatsApp, Signal)
- ✅ Fallback if user loses II access

**Cons:**
- ⚠️ Requires additional password (UX friction)
- ⚠️ Vulnerable if user picks weak password
- ⚠️ Requires trusting API server (though server learns nothing)
- ⚠️ More complex crypto implementation

**Libraries:**
- Rust: `opaque-ke` crate
- TypeScript: `@cloudflare/opaque-ts`

---

### Option 3: Hybrid Approach (VetKeys + Social Recovery)

Combine VetKeys with social recovery for maximum security and UX.

**Architecture:**

```typescript
interface KeyRecoveryMethods {
  // Primary: VetKeys (deterministic from II)
  vetkeys: {
    enabled: true,
    encrypted_seed: string,  // Stored in canister
  },

  // Backup 1: Social recovery (Shamir's Secret Sharing)
  social: {
    enabled: boolean,
    threshold: number,  // e.g., 3 of 5
    guardians: Guardian[],
  },

  // Backup 2: Encrypted backup phrase
  backup_phrase: {
    enabled: boolean,
    encrypted_with_password: string,
  },
}

interface Guardian {
  email?: string,
  principal?: string,
  encrypted_share: string,  // Shamir share, encrypted to guardian
}
```

**Flow:**

1. **Primary recovery**: VetKeys (instant, no user action)
2. **If VetKeys fails**: Social recovery (contact guardians)
3. **Last resort**: Encrypted backup phrase with password

**Pros:**
- ✅ Multiple recovery paths
- ✅ Best UX (VetKeys is instant)
- ✅ Secure fallbacks
- ✅ User chooses their risk/complexity tradeoff

**Cons:**
- ⚠️ Most complex to implement
- ⚠️ Higher maintenance burden

---

### Option 4: Simple Encrypted Backup (Least Secure)

**How it works:**

```typescript
// User sets password
const password = prompt("Set backup password:");

// Derive key from password (PBKDF2/Argon2)
const derivedKey = await deriveKey(password, {
  algorithm: 'PBKDF2',
  iterations: 600_000,
  salt: user_principal,  // Use II principal as salt
});

// Encrypt seed phrase
const encryptedSeed = await encrypt(seedPhrase, derivedKey);

// Store in your database
await db.query(
  "INSERT INTO encrypted_seeds (principal, encrypted_seed) VALUES (?, ?)",
  [principal, encryptedSeed]
);
```

**Pros:**
- ✅ Simple to implement
- ✅ No exotic crypto
- ✅ Works anywhere

**Cons:**
- ⚠️ Weak if user picks bad password
- ⚠️ Backend can brute-force (knows all encrypted seeds)
- ⚠️ Not truly E2EE (server could be compromised)

---

## Recommendation Matrix

| Use Case | Recommended Solution | Why |
|----------|---------------------|-----|
| **ICP-native dApp** | VetKeys (Option 1) | Native integration, fully decentralized, best UX |
| **Multi-chain or API-heavy** | OPRF (Option 2) | Works anywhere, no canister needed |
| **High-security app** | Hybrid (Option 3) | Multiple recovery paths, defense in depth |
| **MVP/Testing** | Simple backup (Option 4) | Quick to build, educate users later |

## Implementation Roadmap for VetKeys (Recommended)

### Phase 1: Basic Implementation (1-2 weeks)

1. **Add VetKeys canister code**
   ```bash
   # Add dependency
   cd ic-canister
   # Motoko
   mops add ic-vetkeys
   # Or Rust
   cargo add ic-vetkeys
   ```

2. **Implement key derivation endpoints**
   - `derive_user_transport_key(principal) -> EncryptedKey`
   - `store_encrypted_seed(principal, encrypted_seed) -> ()`
   - `get_encrypted_seed(principal) -> Option<EncryptedSeed>`

3. **Update frontend auth flow**
   - After II login, check if seed exists
   - If not, generate + encrypt + upload
   - If yes, download + decrypt + restore

### Phase 2: Enhanced UX (1 week)

1. **Add recovery UI**
   - "Backup seed phrase" flow
   - "Restore from cloud" option during login
   - Recovery status indicator

2. **Add social recovery (optional)**
   - Guardian management UI
   - Share distribution flow
   - Recovery request workflow

### Phase 3: Testing & Migration (1 week)

1. **End-to-end testing**
   - Test recovery across browsers
   - Test II re-authentication
   - Test key rotation

2. **Migrate existing users**
   - Detect users without encrypted backup
   - Prompt for backup creation
   - Optional: Force backup for high-value accounts

**Total estimated effort: 3-4 weeks**

## Security Considerations

### VetKeys Security Model

1. **Threat: Subnet takeover**
   - Mitigation: Threshold scheme (33%+ nodes must collude)
   - Probability: Extremely low on production subnets

2. **Threat: User loses II credentials**
   - Mitigation: Always provide seed phrase backup option
   - Recommendation: Force user to save seed phrase before enabling cloud backup

3. **Threat: Canister upgrade attack**
   - Mitigation: Use SNS-controlled canister (decentralized governance)
   - Alternative: Blackholed canister (no upgrades)

### Best Practices

```rust
// Always enforce seed phrase backup
#[update]
async fn store_encrypted_seed(
    principal: Principal,
    encrypted_seed: Vec<u8>,
    user_confirmed_backup: bool,  // <-- Require this!
) -> Result<(), String> {
    if !user_confirmed_backup {
        return Err("Must backup seed phrase before enabling cloud recovery".to_string());
    }

    // Store encrypted seed...
}
```

## Cost Analysis

### VetKeys Costs (ICP)

| Operation | Cycles Cost | USD (est.) |
|-----------|-------------|------------|
| Key derivation | ~100M cycles | ~$0.00013 |
| Store encrypted seed | ~5M cycles | ~$0.000006 |
| Retrieve encrypted seed | ~1M cycles | ~$0.000001 |

**Per user per year**: ~$0.01 (assuming 50 recovery attempts)

### Alternative: API Backend Storage

| Operation | Cost |
|-----------|------|
| Store encrypted seed (PostgreSQL) | ~$0.00001/request |
| Bandwidth (retrieval) | ~$0.000001/request |
| OPRF computation | Negligible (CPU) |

**Per user per year**: ~$0.002

## Conclusion

**For Decent Cloud, I recommend starting with VetKeys** because:

1. ✅ You're already ICP-native (have canisters)
2. ✅ Provides best UX (seamless recovery)
3. ✅ Fully decentralized (matches your ethos)
4. ✅ Future-proof (ICP is investing heavily in vetKeys)
5. ✅ Negligible cost ($0.01/user/year)

**Fallback**: Always require users to backup their seed phrase before enabling cloud recovery. This ensures no single point of failure.

**Next steps**: Would you like me to implement a proof-of-concept VetKeys integration in your canister?
