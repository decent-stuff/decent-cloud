# Account Profiles Design Specification

**Version:** 1.0
**Status:** Implemented
**Created:** 2025-11-18
**Project:** Decent Cloud

## Overview

This document specifies the design for username-based account profiles in Decent Cloud. Each account is identified by a username and can have multiple public keys (for multi-device access). The system uses cryptographic signatures for authentication and will eventually sync critical data to the blockchain ledger.

**Data Structure:** Tree (Account → Keys)
- Each account has 1-10 Ed25519 public keys
- Each key belongs to **exactly one** account (enforced by `UNIQUE(public_key)`)
- No key sharing across accounts (tree structure, not graph)
- Enables multi-device access: laptop, phone, desktop can each have their own key for the same account

## Core Principles

1. **Cryptographic Authentication**: All operations must be cryptographically signed with Ed25519
2. **Fail Fast**: No fallbacks, immediate failure on security violations
3. **Replay Prevention**: Timestamp + nonce ensures requests cannot be replayed
4. **Audit Trail**: All operations logged for forensics and compliance
5. **Soft Deletes**: Preserve historical data, no hard deletes
6. **API-First, Ledger-Later**: Store in API database now, sync critical data to blockchain later

## Account Model

### Username-Based Identity

Each account has:
- **Username**: Unique identifier (3-64 characters, lowercase alphanumeric + underscore/hyphen/period/at-sign)
- **Multiple Public Keys**: 1-10 Ed25519 public keys for multi-device access
- **Equal Key Hierarchy**: All active keys have equal permissions (no "master key")

**Example**: User "alice", "bob.smith", or even "user@example.com" can have separate keys for laptop, phone, and hardware wallet.

### User vs Provider Accounts

- Both use the same underlying account system
- Distinction: Provider accounts can create offerings, user accounts can create contracts
- An account can be both user AND provider simultaneously

## Security Model

### Authentication Flow

Every state-changing request must include:
- **Timestamp**: Client-generated Unix timestamp (nanoseconds)
- **Nonce**: Client-generated UUID v4
- **Signature**: Ed25519 signature over canonical message
- **Public Key**: The key used to sign (must be active key for the account)

### Message Signing Format

```
message = timestamp + nonce + method + path + body
```

**Example**:
```
1700000000000000000550e8400-e29b-41d4-a716-446655440000PUT/api/v1/accounts/alice/profile{"bio":"Hello"}
```

**Note**: Query strings excluded for robustness (non-critical parameters).

### Replay Attack Prevention

**Strategy: Timestamp + Nonce**

1. **Timestamp Validation**:
   - Client generates timestamp locally
   - Backend validates: `|backend_time - user_timestamp| <= 5 minutes`
   - Tolerates clock drift (5 minutes is industry standard)
   - Rejects requests outside time window

2. **Nonce Validation**:
   - Client generates UUID v4 locally
   - Backend checks if nonce seen in last 10 minutes (queries `signature_audit`)
   - If found: reject (replay attack)
   - If not found: accept and insert into `signature_audit`

3. **Performance**:
   - Query `signature_audit` with time-bound index
   - Only check recent data (last 10 minutes)
   - After 10 minutes, timestamp validation rejects request anyway

**Why both?**
- **Timestamp alone**: Attacker can replay within 5-minute window
- **Nonce alone**: Must check against ALL historical nonces (millions of rows)
- **Both together**: Check only last 10 minutes, automatic cleanup

## Username Validation

### Rules
- **Length**: 3-64 characters
- **Characters**: `[a-z0-9._@-]` (lowercase alphanumeric, period, underscore, hyphen, at-sign)
- **Format**: Must start and end with alphanumeric character
- **Normalization**: Convert to lowercase, trim whitespace
- **Regex**: `^[a-z0-9][a-z0-9._@-]{1,62}[a-z0-9]$`
- **Philosophy**: Allows email addresses as usernames but doesn't require or validate email format

### Reserved Usernames
```
["admin", "api", "system", "root", "support", "moderator", "administrator", "test", "null", "undefined", "decent", "cloud"]
```

### Examples
- ✅ Valid: `alice`, `bob123`, `charlie-delta`, `user_99`, `alice.smith`, `user@example.com`, `dev@org`
- ❌ Invalid: `ab` (too short), `-alice` (starts with hyphen), `alice.` (ends with period), `ALICE` (uppercase → normalized to `alice`), `admin` (reserved)

## Database Schema (PostgreSQL)

### Tables

```sql
-- Accounts table
CREATE TABLE accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username TEXT UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT username_format CHECK (username ~ '^[a-z0-9][a-z0-9._@-]*[a-z0-9]$' AND length(username) >= 3 AND length(username) <= 64)
);

CREATE INDEX idx_accounts_username ON accounts(username);

-- Account public keys table
CREATE TABLE account_public_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    public_key BYTEA UNIQUE NOT NULL, -- 32-byte Ed25519 pubkey
    device_name TEXT, -- Optional user-friendly name (e.g., "My iPhone", "Work Laptop")
    is_active BOOLEAN NOT NULL DEFAULT true,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    disabled_at TIMESTAMPTZ,
    disabled_by_key_id UUID REFERENCES account_public_keys(id),
    UNIQUE(account_id, public_key)
);

CREATE INDEX idx_keys_account ON account_public_keys(account_id);
CREATE INDEX idx_keys_pubkey ON account_public_keys(public_key);
CREATE INDEX idx_keys_active ON account_public_keys(account_id, is_active);

-- Signature audit trail
CREATE TABLE signature_audit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID REFERENCES accounts(id),
    action TEXT NOT NULL, -- 'register_account', 'add_key', 'remove_key', etc.
    payload TEXT NOT NULL, -- Full request body (JSON)
    signature BYTEA NOT NULL, -- 64-byte Ed25519 signature
    public_key BYTEA NOT NULL, -- 32-byte Ed25519 pubkey
    timestamp BIGINT NOT NULL, -- Client timestamp (nanoseconds)
    nonce UUID NOT NULL, -- UUID v4
    is_admin_action BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_nonce_time ON signature_audit(nonce, created_at);
CREATE INDEX idx_audit_account ON signature_audit(account_id);
CREATE INDEX idx_audit_created ON signature_audit(created_at);
```

### Constraints

1. **Uniqueness**:
   - Username unique across all accounts
   - Public key unique across all accounts

2. **Business Rules** (enforced in application layer):
   - Max 10 keys per account
   - Min 1 active key per account (cannot remove last key)

3. **Referential Integrity**:
   - Public keys cascade delete with account
   - `disabled_by_key_id` references the key that performed the disable action

## Key Management

### Key Hierarchy

**All keys are equal** - no hierarchy, no "master key" concept.

**Benefits**:
- Simpler mental model
- Any active key can add/remove other keys
- No single point of failure
- Prevents "lost master key = lost account" scenario

### Key Operations

#### Add Key
- Any active key can add a new key
- Max 10 keys per account
- New key becomes immediately active
- Signed by an existing active key

#### Remove Key (Soft Delete)
- Any active key can remove another key (or itself)
- Cannot remove the last active key
- Sets `is_active = 0`, `disabled_at = NOW()`, `disabled_by_key_id`
- Key remains in database for audit trail

#### Key Compromise
- User can remove compromised key using any non-compromised active key
- Admin can also remove key (see Admin Operations)

## API Endpoints (Poem/OpenAPI)

### 1. Register Account

**Endpoint**: `POST /api/v1/accounts`

**Request**:
```json
{
  "username": "alice",
  "publicKey": "0x1234abcd...",
  "timestamp": 1700000000000000000,
  "nonce": "550e8400-e29b-41d4-a716-446655440000",
  "signature": "0xabcd1234..."
}
```

**Signed Message**:
```
1700000000000000000550e8400-e29b-41d4-a716-446655440000POST/api/v1/accounts{"username":"alice","publicKey":"0x1234abcd...","timestamp":1700000000000000000,"nonce":"550e8400-e29b-41d4-a716-446655440000"}
```

**Validation**:
1. Username format validation
2. Username not reserved
3. Username not already taken
4. Public key not already registered
5. Timestamp within 5 minutes
6. Nonce not seen in last 10 minutes
7. Signature valid

**Response** (201 Created):
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "alice",
  "createdAt": 1700000000000000000,
  "publicKeys": [
    {
      "id": "650e8400-e29b-41d4-a716-446655440001",
      "publicKey": "0x1234abcd...",
      "addedAt": 1700000000000000000,
      "isActive": true
    }
  ]
}
```

**Errors**:
- `400 Bad Request`: Invalid username format, reserved username, invalid timestamp
- `409 Conflict`: Username already exists, public key already registered
- `401 Unauthorized`: Invalid signature, replay attack (nonce reused)

### 2. Get Account

**Endpoint**: `GET /api/v1/accounts/:username`

**Response** (200 OK):
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "alice",
  "createdAt": 1700000000000000000,
  "updatedAt": 1700000300000000000,
  "publicKeys": [
    {
      "id": "650e8400-e29b-41d4-a716-446655440001",
      "publicKey": "0x1234abcd...",
      "addedAt": 1700000000000000000,
      "isActive": true
    },
    {
      "id": "750e8400-e29b-41d4-a716-446655440002",
      "publicKey": "0x5678efgh...",
      "addedAt": 1700000300000000000,
      "isActive": true
    }
  ]
}
```

**Errors**:
- `404 Not Found`: Account not found

### 3. Add Public Key

**Endpoint**: `POST /api/v1/accounts/:username/keys`

**Request**:
```json
{
  "newPublicKey": "0x5678efgh...",
  "signingPublicKey": "0x1234abcd...",
  "timestamp": 1700000300000000000,
  "nonce": "550e8400-e29b-41d4-a716-446655440001",
  "signature": "0xefgh5678..."
}
```

**Signed Message**:
```
1700000300000000000550e8400-e29b-41d4-a716-446655440001POST/api/v1/accounts/alice/keys{"newPublicKey":"0x5678efgh...","signingPublicKey":"0x1234abcd...","timestamp":1700000300000000000,"nonce":"550e8400-e29b-41d4-a716-446655440001"}
```

**Validation**:
1. Account exists
2. Signing public key belongs to account and is active
3. New public key not already registered
4. Account has < 10 keys
5. Timestamp within 5 minutes
6. Nonce not seen in last 10 minutes
7. Signature valid

**Response** (201 Created):
```json
{
  "id": "750e8400-e29b-41d4-a716-446655440002",
  "publicKey": "0x5678efgh...",
  "addedAt": 1700000300000000000,
  "isActive": true
}
```

**Errors**:
- `400 Bad Request`: Invalid timestamp, max keys exceeded
- `404 Not Found`: Account not found
- `401 Unauthorized`: Invalid signature, signing key not active, replay attack
- `409 Conflict`: Public key already registered

### 4. Remove Public Key

**Endpoint**: `DELETE /api/v1/accounts/:username/keys/:keyId`

**Request**:
```json
{
  "signingPublicKey": "0x1234abcd...",
  "timestamp": 1700000600000000000,
  "nonce": "550e8400-e29b-41d4-a716-446655440003",
  "signature": "0x9012ijkl..."
}
```

**Signed Message**:
```
1700000600000000000550e8400-e29b-41d4-a716-446655440003DELETE/api/v1/accounts/alice/keys/750e8400-e29b-41d4-a716-446655440002{"signingPublicKey":"0x1234abcd...","timestamp":1700000600000000000,"nonce":"550e8400-e29b-41d4-a716-446655440003"}
```

**Validation**:
1. Account exists
2. Key exists and belongs to account
3. Signing public key belongs to account and is active
4. Key to remove is not the last active key
5. Timestamp within 5 minutes
6. Nonce not seen in last 10 minutes
7. Signature valid

**Action**: Soft delete (set `is_active = 0`, `disabled_at = NOW()`, `disabled_by_key_id`)

**Response** (200 OK):
```json
{
  "id": "750e8400-e29b-41d4-a716-446655440002",
  "publicKey": "0x5678efgh...",
  "addedAt": 1700000300000000000,
  "isActive": false,
  "disabledAt": 1700000600000000000,
  "disabledByKeyId": "650e8400-e29b-41d4-a716-446655440001"
}
```

**Errors**:
- `400 Bad Request`: Cannot remove last active key, invalid timestamp
- `404 Not Found`: Account or key not found
- `401 Unauthorized`: Invalid signature, signing key not active, replay attack

### 5. Update Key Metadata (Device Name)

**Endpoint**: `PUT /api/v1/accounts/:username/keys/:keyId`

**Request**:
```json
{
  "deviceName": "My iPhone 15",
  "signingPublicKey": "0x1234abcd...",
  "timestamp": 1700000700000000000,
  "nonce": "550e8400-e29b-41d4-a716-446655440004",
  "signature": "0x3456mnop..."
}
```

**Signed Message**:
```
1700000700000000000550e8400-e29b-41d4-a716-446655440004PUT/api/v1/accounts/alice/keys/750e8400-e29b-41d4-a716-446655440002{"deviceName":"My iPhone 15","signingPublicKey":"0x1234abcd...","timestamp":1700000700000000000,"nonce":"550e8400-e29b-41d4-a716-446655440004"}
```

**Validation**:
1. Account exists
2. Key exists and belongs to account
3. Signing public key belongs to account and is active
4. Device name ≤ 64 characters (optional, can be null)
5. Timestamp within 5 minutes
6. Nonce not seen in last 10 minutes
7. Signature valid

**Response** (200 OK):
```json
{
  "id": "750e8400-e29b-41d4-a716-446655440002",
  "publicKey": "0x5678efgh...",
  "deviceName": "My iPhone 15",
  "addedAt": 1700000300000000000,
  "isActive": true
}
```

**Errors**:
- `400 Bad Request`: Invalid device name (too long), invalid timestamp
- `404 Not Found`: Account or key not found
- `401 Unauthorized`: Invalid signature, signing key not active, replay attack

**Use Cases**:
- User renames device from "Unnamed Device" to "Work Laptop"
- User corrects typo in device name
- Device name syncs across all browsers (stored in backend, not localStorage)

### 6. Admin: Disable Key

**Endpoint**: `POST /api/v1/admin/accounts/:username/keys/:keyId/disable`

**Authentication**: Requires admin credentials (separate from user key-based auth)

**Request**:
```json
{
  "reason": "Compromised key reported by user"
}
```

**Action**:
- Soft delete key (same as user removal)
- Set `is_admin_action = 1` in `signature_audit`
- Log admin action with reason

**Response** (200 OK):
```json
{
  "id": "750e8400-e29b-41d4-a716-446655440002",
  "publicKey": "0x5678efgh...",
  "isActive": false,
  "disabledAt": 1700000900000000000,
  "disabledByAdmin": true
}
```

**Use Cases**:
- User reports key compromise but has no other active keys
- User loses all keys and needs account recovery
- Security incident response

### 7. Admin: Add Recovery Key

**Endpoint**: `POST /api/v1/admin/accounts/:username/recovery-key`

**Authentication**: Requires admin credentials

**Request**:
```json
{
  "publicKey": "0x9012mnop...",
  "reason": "User lost all keys, verified via support ticket #12345"
}
```

**Action**:
- Add new public key to account
- Set `is_admin_action = 1` in `signature_audit`
- Log admin action with reason

**Response** (201 Created):
```json
{
  "id": "850e8400-e29b-41d4-a716-446655440003",
  "publicKey": "0x9012mnop...",
  "addedAt": 1700001200000000000,
  "isActive": true,
  "addedByAdmin": true
}
```

## Account Recovery

### Scenario: User Loses All Keys

**Process**:
1. User contacts support (out-of-band verification required)
2. Support verifies identity (email, phone, KYC, etc.)
3. Admin uses `POST /api/v1/admin/accounts/:username/recovery-key`
4. Admin provides new public key (user generates new key pair)
5. User can now use new key to manage account
6. Admin action logged in `signature_audit` with `is_admin_action = 1`

**Security Considerations**:
- Admin actions require strong authentication
- All admin actions logged with reason
- Consider multi-signature requirement for admin actions
- Rate limit admin recovery operations

## References

- [Ed25519 Signature Scheme (RFC 8032)](https://tools.ietf.org/html/rfc8032)
- [Poem Framework](https://github.com/poem-web/poem)
- [Decent Cloud Development Guide](./development.md)
