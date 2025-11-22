# Profile Architecture Fix

**Version:** 1.0
**Status:** Planning
**Created:** 2025-11-21
**Project:** Decent Cloud

# Correct Profile Architecture (YAGNI Applied)

Since each account has **exactly one** profile (1:1 relationship), profile fields should be columns in the `accounts` table:

```
Account @alice (accounts table)
  ├─ Authentication fields
  │  ├─ id, username, created_at, updated_at
  │  └─ Device keys (account_public_keys table, 1:many)
  │
  └─ Public Profile fields (same table!)
     ├─ display_name
     ├─ bio
     ├─ avatar_url
     └─ profile_updated_at

  Related data (separate tables, 1:many)
  ├─ Contacts (account_contacts)
  ├─ Socials (account_socials)
  └─ External public keys (account_external_keys - SSH/GPG)
```

**Key Principles:**

1. **YAGNI**: Don't create a separate table for 1:1 relationships
2. **Simplicity**: Fewer tables = fewer JOINs = faster queries
3. **One Account = One Profile**: Profile fields are nullable columns in accounts
4. **Any Device Key Can Edit**: All active device keys can edit the account's profile
5. **Clear Separation**:
   - Account = Authentication (device keys, username, account ID)
   - Profile = Public presentation (display_name, bio, avatar - same row)

## Database Changes

### Schema Comparison

**Before (Broken):**
```sql
-- Separate table for profiles (1:1 with ???)
CREATE TABLE user_profiles (
    pubkey BLOB NOT NULL UNIQUE,  -- Which pubkey if account has 3 keys?
    display_name TEXT,
    bio TEXT,
    avatar_url TEXT
);

-- Profile-related data keyed by pubkey
CREATE TABLE user_contacts (user_pubkey BLOB, ...);
CREATE TABLE user_socials (user_pubkey BLOB, ...);
CREATE TABLE user_public_keys (user_pubkey BLOB, ...);
```

**After (Fixed - YAGNI):**
```sql
-- Profile fields in accounts table (1:1 relationship)
CREATE TABLE accounts (
    id BLOB PRIMARY KEY,
    username TEXT UNIQUE,
    created_at INTEGER,
    updated_at INTEGER,
    -- Profile fields (nullable)
    display_name TEXT,
    bio TEXT,
    avatar_url TEXT,
    profile_updated_at INTEGER
);

-- Related data keyed by account_id (1:many)
CREATE TABLE account_contacts (account_id BLOB, ...);
CREATE TABLE account_socials (account_id BLOB, ...);
CREATE TABLE account_external_keys (account_id BLOB, ...);  -- SSH/GPG, not auth keys
```

### Migration: Add Profile Fields to `accounts` Table

```sql
-- Step 1: Add profile columns to accounts table
ALTER TABLE accounts ADD COLUMN display_name TEXT;
ALTER TABLE accounts ADD COLUMN bio TEXT;
ALTER TABLE accounts ADD COLUMN avatar_url TEXT;
ALTER TABLE accounts ADD COLUMN profile_updated_at INTEGER;

-- Step 2: Migrate existing data from user_profiles
-- Strategy: For each user_profiles entry, find the account that owns that pubkey
UPDATE accounts
SET
    display_name = (
        SELECT up.display_name
        FROM user_profiles up
        INNER JOIN account_public_keys apk ON up.pubkey = apk.public_key
        WHERE apk.account_id = accounts.id
        LIMIT 1
    ),
    bio = (
        SELECT up.bio
        FROM user_profiles up
        INNER JOIN account_public_keys apk ON up.pubkey = apk.public_key
        WHERE apk.account_id = accounts.id
        LIMIT 1
    ),
    avatar_url = (
        SELECT up.avatar_url
        FROM user_profiles up
        INNER JOIN account_public_keys apk ON up.pubkey = apk.public_key
        WHERE apk.account_id = accounts.id
        LIMIT 1
    ),
    profile_updated_at = (
        SELECT up.updated_at_ns
        FROM user_profiles up
        INNER JOIN account_public_keys apk ON up.pubkey = apk.public_key
        WHERE apk.account_id = accounts.id
        LIMIT 1
    )
WHERE EXISTS (
    SELECT 1
    FROM user_profiles up
    INNER JOIN account_public_keys apk ON up.pubkey = apk.public_key
    WHERE apk.account_id = accounts.id
);

-- Step 3: Drop old table (after verification)
-- DROP TABLE user_profiles;
```

**Benefits of this approach:**
- No JOINs needed to fetch profile data
- Atomic updates (account + profile in one transaction)
- Simpler schema (one less table)
- Better performance (fewer queries)

### Update Related Tables

All profile-related tables need to reference `account_id` instead of `pubkey`:

```sql
-- user_contacts → account_contacts
ALTER TABLE user_contacts RENAME TO old_user_contacts;

CREATE TABLE account_contacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id BLOB NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    contact_type TEXT NOT NULL,
    contact_value TEXT NOT NULL,
    verified BOOLEAN DEFAULT FALSE,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000)
);

CREATE INDEX idx_account_contacts_account_id ON account_contacts(account_id);

-- Migrate data
INSERT INTO account_contacts (account_id, contact_type, contact_value, verified, created_at)
SELECT DISTINCT
    apk.account_id,
    ouc.contact_type,
    ouc.contact_value,
    ouc.verified,
    ouc.created_at_ns
FROM old_user_contacts ouc
INNER JOIN account_public_keys apk ON ouc.user_pubkey = apk.public_key;

-- user_socials → account_socials
ALTER TABLE user_socials RENAME TO old_user_socials;

CREATE TABLE account_socials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id BLOB NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    username TEXT NOT NULL,
    profile_url TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000)
);

CREATE INDEX idx_account_socials_account_id ON account_socials(account_id);

-- Migrate data
INSERT INTO account_socials (account_id, platform, username, profile_url, created_at)
SELECT DISTINCT
    apk.account_id,
    ous.platform,
    ous.username,
    ous.profile_url,
    ous.created_at_ns
FROM old_user_socials ous
INNER JOIN account_public_keys apk ON ous.user_pubkey = apk.public_key;

-- user_public_keys → account_external_keys (renamed to avoid confusion with auth keys)
ALTER TABLE user_public_keys RENAME TO old_user_public_keys;

CREATE TABLE account_external_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id BLOB NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    key_type TEXT NOT NULL,  -- ssh-ed25519, ssh-rsa, gpg, secp256k1, etc.
    key_data TEXT NOT NULL,
    key_fingerprint TEXT,
    label TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000)
);

CREATE INDEX idx_account_external_keys_account_id ON account_external_keys(account_id);

-- Migrate data
INSERT INTO account_external_keys (account_id, key_type, key_data, key_fingerprint, label, created_at)
SELECT DISTINCT
    apk.account_id,
    oupk.key_type,
    oupk.key_data,
    oupk.key_fingerprint,
    oupk.label,
    oupk.created_at_ns
FROM old_user_public_keys oupk
INNER JOIN account_public_keys apk ON oupk.user_pubkey = apk.public_key;

-- Drop old tables after verification
-- DROP TABLE old_user_contacts;
-- DROP TABLE old_user_socials;
-- DROP TABLE old_user_public_keys;
```

### Keep `user_registrations` and `user_activity`

These are legitimately keyed by pubkey (from blockchain ledger):
- `user_registrations`: Historical blockchain registration records
- Provider/requester activity: Tied to blockchain pubkey, not account system

## API Changes

### Before (Broken)

```
GET  /api/v1/users/:pubkey/profile          # Which pubkey if account has 3 keys?
PUT  /api/v1/users/:pubkey/profile          # Must use signing key = same pubkey
POST /api/v1/users/:pubkey/contacts
```

### After (Fixed)

```
GET  /api/v1/accounts/:username/profile     # Account-based, unambiguous
PUT  /api/v1/accounts/:username/profile     # Any active key can edit
POST /api/v1/accounts/:username/contacts
GET  /api/v1/accounts/:username/socials
POST /api/v1/accounts/:username/socials
GET  /api/v1/accounts/:username/public-keys # External keys (SSH/GPG)
POST /api/v1/accounts/:username/public-keys
```

### Public Profile View (Read-Only)

For viewing other users' profiles:

```
GET /api/v1/profiles/:username              # Public read-only view
```

Returns combined public data:
- Account profile (display_name, bio, avatar_url from accounts table)
- Contacts (verified only)
- Social accounts
- External public keys (SSH/GPG from account_external_keys)
- User activity (offerings, rentals)

## UI/UX Changes

### New Dashboard Structure

```
/dashboard
  ├─ /account                    # Authentication & Security
  │  ├─ Account Overview
  │  │  ├─ Username
  │  │  ├─ Account ID
  │  │  ├─ Created date
  │  │  └─ Active device count
  │  └─ Device Management
  │     ├─ List devices (with names)
  │     ├─ Add device
  │     └─ Remove device
  │
  └─ /profile                    # Public Profile
     ├─ Basic Info
     │  ├─ Display name
     │  ├─ Bio
     │  └─ Avatar URL
     ├─ Contact Info
     ├─ Social Accounts
     └─ Public Keys (SSH/GPG)
```

### Page Renaming

**Before:**
- `/dashboard/profile` → "Profile Settings" (confusing)
- `/dashboard/account` → "Account Settings" (overlapping)

**After:**
- `/dashboard/account` → "Account & Security" (authentication, device keys)
- `/dashboard/profile` → "Public Profile" (what others see)

### Page Descriptions

**Account & Security:**
> "Manage your account credentials and device access. Control which devices can sign in to your account."

**Public Profile:**
> "Information visible to other users. This is how you appear to providers and renters on the platform."

### Profile Page Improvements

1. **Clear Privacy Indicator**: Show "👁️ Visible to everyone" prominently
2. **Preview Button**: "See how others see your profile"
3. **No Signing Key Required**: Any active device key can edit (simplify the UX)

## Implementation Plan

### Phase 1: Database Migration

1. Create migration `003_account_profiles_fix.sql`
2. Add profile columns to `accounts` table:
   - `display_name TEXT`
   - `bio TEXT`
   - `avatar_url TEXT`
   - `profile_updated_at INTEGER`
3. Create new account-based tables:
   - `account_contacts` (from user_contacts)
   - `account_socials` (from user_socials)
   - `account_external_keys` (from user_public_keys)
4. Migrate data from old pubkey-based tables to new account-based tables
5. Test migration with cargo make

### Phase 2: Backend API Updates

1. Update database structs and methods in `api/src/database/`:
   - Add profile fields to `Account` struct in `accounts.rs`
   - Rename `users.rs` → `profiles.rs` (clearer naming)
   - Update methods:
     - `update_account_profile(account_id, display_name, bio, avatar_url)` (simple UPDATE)
     - `get_account_contacts(account_id)` instead of `get_user_contacts(pubkey)`
     - `get_account_socials(account_id)` instead of `get_user_socials(pubkey)`
     - `get_account_external_keys(account_id)` instead of `get_user_public_keys(pubkey)`
2. Update API endpoints in `api/src/openapi/users.rs`:
   - Change routes from `/users/:pubkey/*` to `/accounts/:username/*`
   - Update authentication to accept ANY active key from account
   - Add `/profiles/:username` for public read-only view
3. Update Rust types exported to TypeScript
4. Update tests to use account-based approach
5. Run `cargo make` to verify

### Phase 3: Frontend Updates

1. Update API client (`website/src/lib/services/user-api.ts`):
   - Change endpoints to use username instead of pubkey
   - Remove pubkey parameter from all methods
   - Update TypeScript types from generated Rust types
2. Update components:
   - `UserProfileEditor.svelte`: Use currentIdentity.account.username, remove signing key requirement
   - `AccountOverview.svelte`: Keep as-is (already correct)
   - `ContactsEditor.svelte`: Use account username instead of pubkey
   - `SocialsEditor.svelte`: Use account username instead of pubkey
   - `PublicKeysEditor.svelte` → `ExternalKeysEditor.svelte` (clearer naming, SSH/GPG keys)
3. Update pages:
   - `/dashboard/profile/+page.svelte`: Simplify, use account username, any active key can edit
   - `/dashboard/account/+page.svelte`: Update title to "Account & Security"
4. Add public profile view route:
   - `/profiles/[username]/+page.svelte`: Public read-only profile view

### Phase 4: User Activity Integration

Currently `/dashboard/user/[pubkey]` shows blockchain activity. This is correct (blockchain uses pubkey).

**Enhancement**: Link to account profile if one exists:
```
GET /api/v1/profiles/by-pubkey/:pubkey
  → Returns account profile if pubkey is registered to an account
  → Returns null if pubkey only exists on blockchain (no account)
```

This allows:
```svelte
{#if userProfile}
  <a href="/profiles/{userProfile.username}">@{userProfile.username}</a>
{:else}
  <span>{truncatedPubkey}</span>
{/if}
```

## Testing Requirements

### Database Tests

- [ ] Test migration: profile fields added to accounts table
- [ ] Test data migration from user_profiles to accounts.display_name/bio/avatar_url
- [ ] Test account_contacts/socials/external_keys foreign key constraints (CASCADE DELETE)
- [ ] Test account with multiple device keys editing same profile
- [ ] Test updating profile doesn't affect device keys
- [ ] Test deleting account cascades to contacts/socials/external_keys

### API Tests

- [ ] Test profile CRUD with different device keys from same account
- [ ] Test authentication: any active key can edit profile
- [ ] Test public profile endpoint (read-only, no auth required)
- [ ] Test error cases (profile not found, account not found)

### Integration Tests

- [ ] Test multi-device scenario:
  1. Create account with key A
  2. Add key B to account
  3. Use key B to edit profile
  4. Verify profile visible via account username
- [ ] Test profile visibility to other users
- [ ] Test device removal doesn't orphan profile

### UI Tests

- [ ] Test profile editing flow
- [ ] Test public profile view
- [ ] Test account/profile page clarity (users understand difference)

## Migration Path for Users

### No Action Required

Users with existing profiles will have data automatically migrated:
1. System finds pubkey → account mapping via account_public_keys
2. Copies profile data (display_name, bio, avatar_url) to accounts table
3. Migrates contacts, socials, external keys to new account-based tables
4. Old data preserved, new structure enforced

### Edge Cases

**Multiple profiles for same account** (if bug allowed):
- Migration script takes FIRST profile found (by updated_at)
- Log warning for manual review
- Contact affected users if any

**Profiles with no account** (orphaned):
- Keep in old table temporarily
- Log for manual review
- Likely blockchain-only users, no account registered

## Rollout Plan

1. **Development**: Implement and test thoroughly
2. **Staging**: Run migration on staging DB, verify data integrity
3. **Production**:
   - Run migration during low-traffic window
   - Keep old tables for 30 days (rollback safety)
   - Monitor error logs for issues
4. **Cleanup**: Drop old tables after verification period

## Success Criteria

- [ ] Every account has exactly ONE profile (enforced by schema)
- [ ] Profile editing works with ANY active device key
- [ ] UI clearly distinguishes Account (auth) from Profile (public)
- [ ] Public profile accessible via username
- [ ] All existing profile data preserved
- [ ] Zero data loss during migration
- [ ] `cargo make` passes with zero warnings
- [ ] All tests green

## Key Benefits Summary

**Simplicity (YAGNI):**
- ✅ No separate `account_profiles` table for 1:1 relationship
- ✅ Profile fields are just nullable columns in `accounts`
- ✅ One less table to maintain
- ✅ Fewer JOINs needed

**Performance:**
- ✅ Fetching profile = fetching account (single query)
- ✅ Updating profile = simple UPDATE on accounts table
- ✅ No JOIN overhead for basic profile data

**Data Integrity:**
- ✅ Impossible to have orphaned profiles (profile IS the account)
- ✅ Impossible to have multiple profiles per account
- ✅ Cascade deletes work correctly (contacts/socials deleted with account)

**Developer Experience:**
- ✅ Simpler mental model: account has display_name, just like it has username
- ✅ Less code: no separate profile repository
- ✅ Clearer API: `/accounts/:username/profile` returns account with profile fields
- ✅ TypeScript types simpler: `Account` includes profile fields

**User Experience:**
- ✅ Clear separation: "Account & Security" vs "Public Profile"
- ✅ Any device can edit profile (no confusing "signing key" requirement)
- ✅ Profile always tied to account username (consistent identity)

## References

- [Account Profiles Design](./ACCOUNT_PROFILES_DESIGN.md)
- [Web Auth UX Redesign](./WEB_AUTH_UX_REDESIGN.md)
- Database schema: `api/migrations/001_original_schema.sql`
- Account schema: `api/migrations/002_account_profiles.sql`
