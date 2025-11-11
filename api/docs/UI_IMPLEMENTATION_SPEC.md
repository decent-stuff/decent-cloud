# UI Implementation Specification for User Profile Management

## Overview

This document specifies the frontend changes required to support user profile management with authenticated API endpoints. The backend now provides Ed25519 signature-based authentication and full CRUD operations for user profiles, contacts, social accounts, and public keys.

## Authentication Implementation

### 1. Signing Request Helper Function

Create a utility function to sign API requests:

```typescript
// lib/services/auth-api.ts

import { sha512 } from '@noble/hashes/sha512';
import type { Ed25519KeyIdentity } from '@dfinity/identity';

export interface SignedRequest {
  headers: {
    'X-Public-Key': string;
    'X-Signature': string;
    'X-Timestamp': string;
    'Content-Type': string;
  };
  body: string;
}

/**
 * Sign an API request with Ed25519 key
 * Message format: timestamp + method + path + body
 */
export async function signRequest(
  identity: Ed25519KeyIdentity,
  method: string,
  path: string,
  bodyData?: any
): Promise<SignedRequest> {
  const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
  const secretKeyBytes = new Uint8Array(identity.getKeyPair().secretKey);

  // Get current timestamp in nanoseconds
  const timestampNs = (Date.now() * 1_000_000).toString();

  // Serialize body
  const body = bodyData ? JSON.stringify(bodyData) : '';

  // Construct message: timestamp + method + path + body
  const message = new TextEncoder().encode(timestampNs + method + path + body);

  // Sign message (Ed25519 with SHA-512 prehashing)
  const prehashed = sha512(message);

  // Note: This is simplified - use the actual Ed25519 signing from @dfinity/identity
  // The identity object should have a sign method or you can use ed25519-dalek equivalent
  const signature = await signWithEd25519(secretKeyBytes, prehashed);

  return {
    headers: {
      'X-Public-Key': bytesToHex(publicKeyBytes),
      'X-Signature': bytesToHex(signature),
      'X-Timestamp': timestampNs,
      'Content-Type': 'application/json',
    },
    body,
  };
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

// This needs proper implementation using @noble/ed25519 or similar
async function signWithEd25519(secretKey: Uint8Array, message: Uint8Array): Promise<Uint8Array> {
  // TODO: Implement actual Ed25519 signing with context
  // Backend expects: identity.sign() which uses SHA-512 prehashing
  // and 'DecentCloudContext' context string
  throw new Error('Implement Ed25519 signing');
}
```

### 2. Authenticated API Client

Create a client for authenticated requests:

```typescript
// lib/services/user-api.ts

import { signRequest } from './auth-api';
import type { Ed25519KeyIdentity } from '@dfinity/identity';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'https://api.decentcloud.org';

export class UserApiClient {
  constructor(private signingIdentity: Ed25519KeyIdentity) {}

  private async authenticatedFetch(
    method: string,
    path: string,
    body?: any
  ): Promise<Response> {
    const { headers, body: signedBody } = await signRequest(
      this.signingIdentity,
      method,
      path,
      body
    );

    return fetch(`${API_BASE}${path}`, {
      method,
      headers,
      body: signedBody || undefined,
    });
  }

  // Profile
  async updateProfile(pubkey: string, profile: {
    display_name?: string;
    bio?: string;
    avatar_url?: string;
  }) {
    const path = `/api/v1/users/${pubkey}/profile`;
    return this.authenticatedFetch('PUT', path, profile);
  }

  // Contacts
  async upsertContact(pubkey: string, contact: {
    contact_type: string;
    contact_value: string;
    verified?: boolean;
  }) {
    const path = `/api/v1/users/${pubkey}/contacts`;
    return this.authenticatedFetch('POST', path, contact);
  }

  async deleteContact(pubkey: string, contactType: string) {
    const path = `/api/v1/users/${pubkey}/contacts/${contactType}`;
    return this.authenticatedFetch('DELETE', path);
  }

  // Socials
  async upsertSocial(pubkey: string, social: {
    platform: string;
    username: string;
    profile_url?: string;
  }) {
    const path = `/api/v1/users/${pubkey}/socials`;
    return this.authenticatedFetch('POST', path, social);
  }

  async deleteSocial(pubkey: string, platform: string) {
    const path = `/api/v1/users/${pubkey}/socials/${platform}`;
    return this.authenticatedFetch('DELETE', path);
  }

  // Public Keys
  async addPublicKey(pubkey: string, key: {
    key_type: string;
    key_data: string;
    key_fingerprint?: string;
    label?: string;
  }) {
    const path = `/api/v1/users/${pubkey}/keys`;
    return this.authenticatedFetch('POST', path, key);
  }

  async deletePublicKey(pubkey: string, fingerprint: string) {
    const path = `/api/v1/users/${pubkey}/keys/${fingerprint}`;
    return this.authenticatedFetch('DELETE', path);
  }
}
```

## Required UI Components

### 3. User Profile Page

Create `/dashboard/profile` page:

```typescript
// app/dashboard/profile/page.tsx

import { UserProfileEditor } from '@/components/UserProfileEditor';
import { authStore } from '@/lib/stores/auth';

export default function ProfilePage() {
  const { currentIdentity, signingIdentity } = authStore;

  if (!signingIdentity) {
    return (
      <div className="p-8">
        <h1 className="text-2xl font-bold mb-4">Profile Settings</h1>
        <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-4">
          <p className="text-yellow-800">
            You need a signing key (seed phrase identity) to edit your profile.
          </p>
          <button className="mt-4 px-4 py-2 bg-yellow-600 text-white rounded">
            Create Signing Key
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8 max-w-4xl mx-auto">
      <h1 className="text-2xl font-bold mb-6">Profile Settings</h1>
      <UserProfileEditor
        identity={currentIdentity}
        signingIdentity={signingIdentity}
      />
    </div>
  );
}
```

### 4. Profile Editor Component

```typescript
// components/UserProfileEditor.tsx

import { useState, useEffect } from 'react';
import { UserApiClient } from '@/lib/services/user-api';
import type { IdentityInfo } from '@/lib/stores/auth';

interface Props {
  identity: IdentityInfo;
  signingIdentity: IdentityInfo;
}

export function UserProfileEditor({ identity, signingIdentity }: Props) {
  const [profile, setProfile] = useState({
    display_name: '',
    bio: '',
    avatar_url: '',
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const pubkey = Buffer.from(signingIdentity.publicKeyBytes!).toString('hex');
  const apiClient = new UserApiClient(signingIdentity.identity as any);

  // Fetch existing profile
  useEffect(() => {
    async function loadProfile() {
      try {
        const res = await fetch(`/api/v1/users/${pubkey}/profile`);
        if (res.ok) {
          const data = await res.json();
          if (data.success && data.data) {
            setProfile({
              display_name: data.data.display_name || '',
              bio: data.data.bio || '',
              avatar_url: data.data.avatar_url || '',
            });
          }
        }
      } catch (err) {
        console.error('Failed to load profile:', err);
      }
    }
    loadProfile();
  }, [pubkey]);

  async function handleSave() {
    setLoading(true);
    setError(null);

    try {
      const res = await apiClient.updateProfile(pubkey, profile);
      const data = await res.json();

      if (!data.success) {
        throw new Error(data.error || 'Failed to update profile');
      }

      alert('Profile updated successfully!');
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="space-y-6">
      <div className="bg-white border rounded-lg p-6">
        <h2 className="text-xl font-semibold mb-4">Basic Information</h2>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-2">
              Display Name
            </label>
            <input
              type="text"
              value={profile.display_name}
              onChange={(e) => setProfile({ ...profile, display_name: e.target.value })}
              className="w-full px-3 py-2 border rounded-lg"
              placeholder="Your display name"
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">Bio</label>
            <textarea
              value={profile.bio}
              onChange={(e) => setProfile({ ...profile, bio: e.target.value })}
              className="w-full px-3 py-2 border rounded-lg"
              rows={4}
              placeholder="Tell us about yourself"
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">Avatar URL</label>
            <input
              type="url"
              value={profile.avatar_url}
              onChange={(e) => setProfile({ ...profile, avatar_url: e.target.value })}
              className="w-full px-3 py-2 border rounded-lg"
              placeholder="https://example.com/avatar.png"
            />
          </div>
        </div>

        {error && (
          <div className="mt-4 p-3 bg-red-50 border border-red-200 rounded text-red-700">
            {error}
          </div>
        )}

        <button
          onClick={handleSave}
          disabled={loading}
          className="mt-6 px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
        >
          {loading ? 'Saving...' : 'Save Profile'}
        </button>
      </div>

      <ContactsEditor pubkey={pubkey} apiClient={apiClient} />
      <SocialsEditor pubkey={pubkey} apiClient={apiClient} />
      <PublicKeysEditor pubkey={pubkey} apiClient={apiClient} />
    </div>
  );
}
```

### 5. Contacts Editor Component

```typescript
// components/ContactsEditor.tsx

interface Contact {
  contact_type: string;
  contact_value: string;
  verified: boolean;
}

export function ContactsEditor({ pubkey, apiClient }: {
  pubkey: string;
  apiClient: UserApiClient;
}) {
  const [contacts, setContacts] = useState<Contact[]>([]);
  const [newContact, setNewContact] = useState({ type: 'email', value: '' });
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    loadContacts();
  }, [pubkey]);

  async function loadContacts() {
    const res = await fetch(`/api/v1/users/${pubkey}/contacts`);
    if (res.ok) {
      const data = await res.json();
      if (data.success) setContacts(data.data);
    }
  }

  async function handleAdd() {
    setLoading(true);
    try {
      await apiClient.upsertContact(pubkey, {
        contact_type: newContact.type,
        contact_value: newContact.value,
      });
      setNewContact({ type: 'email', value: '' });
      await loadContacts();
    } catch (err) {
      alert('Failed to add contact');
    } finally {
      setLoading(false);
    }
  }

  async function handleDelete(type: string) {
    if (!confirm(`Delete ${type} contact?`)) return;

    try {
      await apiClient.deleteContact(pubkey, type);
      await loadContacts();
    } catch (err) {
      alert('Failed to delete contact');
    }
  }

  return (
    <div className="bg-white border rounded-lg p-6">
      <h2 className="text-xl font-semibold mb-4">Contact Information</h2>

      {/* Contact list */}
      <div className="space-y-2 mb-4">
        {contacts.map((contact) => (
          <div key={contact.contact_type} className="flex items-center justify-between p-3 bg-gray-50 rounded">
            <div>
              <span className="font-medium">{contact.contact_type}:</span>{' '}
              {contact.contact_value}
              {contact.verified && (
                <span className="ml-2 text-xs bg-green-100 text-green-800 px-2 py-1 rounded">
                  Verified
                </span>
              )}
            </div>
            <button
              onClick={() => handleDelete(contact.contact_type)}
              className="text-red-600 hover:text-red-800"
            >
              Delete
            </button>
          </div>
        ))}
      </div>

      {/* Add new contact */}
      <div className="flex gap-2">
        <select
          value={newContact.type}
          onChange={(e) => setNewContact({ ...newContact, type: e.target.value })}
          className="px-3 py-2 border rounded-lg"
        >
          <option value="email">Email</option>
          <option value="phone">Phone</option>
          <option value="telegram">Telegram</option>
        </select>
        <input
          type="text"
          value={newContact.value}
          onChange={(e) => setNewContact({ ...newContact, value: e.target.value })}
          className="flex-1 px-3 py-2 border rounded-lg"
          placeholder="Contact value"
        />
        <button
          onClick={handleAdd}
          disabled={!newContact.value || loading}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
        >
          Add
        </button>
      </div>
    </div>
  );
}
```

### 6. Socials Editor Component

Similar to ContactsEditor but for social platforms:

```typescript
// components/SocialsEditor.tsx

interface Social {
  platform: string;
  username: string;
  profile_url: string | null;
}

export function SocialsEditor({ pubkey, apiClient }: {
  pubkey: string;
  apiClient: UserApiClient;
}) {
  const [socials, setSocials] = useState<Social[]>([]);
  const [newSocial, setNewSocial] = useState({ platform: 'twitter', username: '' });

  // Similar implementation to ContactsEditor
  // Load socials from /api/v1/users/:pubkey/socials
  // Add via apiClient.upsertSocial()
  // Delete via apiClient.deleteSocial()

  return (
    <div className="bg-white border rounded-lg p-6">
      <h2 className="text-xl font-semibold mb-4">Social Media</h2>
      {/* Implementation */}
    </div>
  );
}
```

### 7. Public Keys Editor Component

```typescript
// components/PublicKeysEditor.tsx

interface PublicKey {
  key_type: string;
  key_data: string;
  key_fingerprint: string | null;
  label: string | null;
}

export function PublicKeysEditor({ pubkey, apiClient }: {
  pubkey: string;
  apiClient: UserApiClient;
}) {
  const [keys, setKeys] = useState<PublicKey[]>([]);
  const [newKey, setNewKey] = useState({
    type: 'ssh-ed25519',
    data: '',
    label: '',
  });

  // Load keys from /api/v1/users/:pubkey/keys
  // Add via apiClient.addPublicKey()
  // Delete via apiClient.deletePublicKey()

  return (
    <div className="bg-white border rounded-lg p-6">
      <h2 className="text-xl font-semibold mb-4">Public Keys</h2>
      {/* Implementation */}
    </div>
  );
}
```

## Critical Requirements

### 8. Enforce Signing Key Creation

Update the auth flow to ensure users have a signing key:

```typescript
// lib/stores/auth.ts

// In the initialize() function, after II login:
if (type === 'ii' && !foundSigningIdentity) {
  // Force seed phrase creation
  const newSeedPhrase = generateNewSeedPhrase();
  const newIdentity = identityFromSeed(newSeedPhrase);
  // ... store and set as signingIdentity

  // Show mandatory backup dialog
  showSeedPhrase.set(true);
  showBackupInstructions.set(true);
}
```

### 9. Add Navigation Link

Add profile link to dashboard navigation:

```typescript
// components/DashboardNav.tsx

<nav>
  <Link href="/dashboard">Dashboard</Link>
  <Link href="/dashboard/profile">Profile</Link>  {/* NEW */}
  <Link href="/dashboard/contracts">Contracts</Link>
  {/* ... */}
</nav>
```

## Testing Checklist

- [ ] User can create/update profile with display name, bio, avatar
- [ ] User can add/remove contacts (email, phone, telegram)
- [ ] User can add/remove social accounts (twitter, github, discord)
- [ ] User can add/remove public keys (SSH, GPG)
- [ ] Signature verification works (requests are authenticated)
- [ ] Users without signing key are prompted to create one
- [ ] Unauthorized requests (wrong pubkey) are rejected with 401
- [ ] Expired timestamps (>5 min old) are rejected
- [ ] Profile data displays correctly after update

## API Endpoints Reference

### Read (Unauthenticated)
- `GET /api/v1/users/:pubkey/profile` - Get profile
- `GET /api/v1/users/:pubkey/contacts` - List contacts
- `GET /api/v1/users/:pubkey/socials` - List socials
- `GET /api/v1/users/:pubkey/keys` - List public keys

### Write (Authenticated)
- `PUT /api/v1/users/:pubkey/profile` - Update profile
- `POST /api/v1/users/:pubkey/contacts` - Upsert contact
- `DELETE /api/v1/users/:pubkey/contacts/:type` - Delete contact
- `POST /api/v1/users/:pubkey/socials` - Upsert social
- `DELETE /api/v1/users/:pubkey/socials/:platform` - Delete social
- `POST /api/v1/users/:pubkey/keys` - Add public key
- `DELETE /api/v1/users/:pubkey/keys/:fingerprint` - Delete public key

## Authentication Headers (Required for Write Operations)

```
X-Public-Key: <hex-encoded-32-byte-pubkey>
X-Signature: <hex-encoded-64-byte-ed25519-signature>
X-Timestamp: <unix-timestamp-in-nanoseconds>
Content-Type: application/json
```

Message to sign: `timestamp + method + path + body`

Example:
```
Message: "1699564800000000000PUT/api/v1/users/abc123/profile{\"display_name\":\"Alice\"}"
```

## Next Steps

1. Implement `signRequest()` function with proper Ed25519 signing
2. Create `UserApiClient` class
3. Build profile page with all editor components
4. Add profile link to navigation
5. Test authentication flow end-to-end
6. Handle error cases (expired timestamps, invalid signatures)
7. Add loading states and user feedback
