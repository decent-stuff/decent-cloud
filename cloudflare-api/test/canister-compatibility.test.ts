import { describe, it, expect, beforeEach } from 'vitest';
import { unstable_dev } from 'wrangler';
import type { UnstableDevWorker } from 'wrangler';

describe('Canister Compatibility Tests', () => {
  let worker: UnstableDevWorker;

  beforeAll(async () => {
    worker = await unstable_dev('src/index.ts', {
      experimental: { disableExperimentalWarning: true },
      vars: {
        ENVIRONMENT: 'test',
        TEST_DB_NAME: 'test-canister-compat'
      }
    });
  });

  beforeEach(async () => {
    // Clean up test data
    const testDb = (worker as any).env.TEST_DB;
    await testDb.prepare('DELETE FROM dc_users').run();
    await testDb.prepare('DELETE FROM provider_profiles').run();
    await testDb.prepare('DELETE FROM provider_offerings').run();
  });

  it('should handle provider registration', async () => {
    const pubkey = Array.from({ length: 32 }, () => Math.floor(Math.random() * 256));
    const signature = Array.from({ length: 64 }, () => Math.floor(Math.random() * 256));

    const response = await worker.fetch('/api/v1/canister/provider_register_anonymous', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        args: [pubkey, signature, null]
      })
    });

    expect(response.status).toBe(200);
    const result = await response.json();
    expect(result.success).toBe(true);
    expect(result.data).toHaveProperty('Ok');
    expect(result.data.Ok).toContain('queued');
  });

  it('should handle provider profile updates', async () => {
    const pubkey = Array.from({ length: 32 }, () => Math.floor(Math.random() * 256));
    const profileData = Array.from({ length: 100 }, () => Math.floor(Math.random() * 256));
    const signature = Array.from({ length: 64 }, () => Math.floor(Math.random() * 256));

    const response = await worker.fetch('/api/v1/canister/provider_update_profile_anonymous', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        args: [pubkey, profileData, signature, null]
      })
    });

    expect(response.status).toBe(200);
    const result = await response.json();
    expect(result.success).toBe(true);
    expect(result.data).toHaveProperty('Ok');
  });

  it('should retrieve cached provider profile', async () => {
    const pubkey = Array.from({ length: 32 }, () => Math.floor(Math.random() * 256));
    const pubkeyHex = pubkey.map(b => b.toString(16).padStart(2, '0')).join('');
    const profileData = new TextEncoder().encode('test profile data');

    // First, create a profile in D1 directly
    const testDb = (worker as any).env.TEST_DB;
    await testDb.prepare(`
      INSERT INTO provider_profiles (pubkey, profile_data, version, created_at, updated_at)
      VALUES (?, ?, ?, datetime('now'), datetime('now'))
    `).bind(pubkeyHex, profileData, 1).run();

    // Then try to retrieve via canister-compatible API
    const response = await worker.fetch('/api/v1/canister/provider_get_profile_by_pubkey_bytes', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        args: [pubkey]
      })
    });

    expect(response.status).toBe(200);
    const result = await response.json();
    expect(result.success).toBe(true);
    expect(result.data).toBe('test profile data');
  });

  it('should handle offering search', async () => {
    const response = await worker.fetch('/api/v1/canister/offering_search', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        args: ['test query']
      })
    });

    expect(response.status).toBe(200);
    const result = await response.json();
    expect(result.success).toBe(true);
    expect(Array.isArray(result.data)).toBe(true);
  });

  it('should return registration fee', async () => {
    const response = await worker.fetch('/api/v1/canister/get_registration_fee', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        args: []
      })
    });

    expect(response.status).toBe(200);
    const result = await response.json();
    expect(result.success).toBe(true);
    expect(typeof result.data).toBe('string'); // BigInt serialized as string
  });

  it('should handle check-in nonce generation', async () => {
    const response = await worker.fetch('/api/v1/canister/get_check_in_nonce', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        args: []
      })
    });

    expect(response.status).toBe(200);
    const result = await response.json();
    expect(result.success).toBe(true);
    expect(Array.isArray(result.data)).toBe(true);
    expect(result.data.length).toBe(32); // 32 bytes nonce
  });

  it('should handle errors gracefully', async () => {
    const response = await worker.fetch('/api/v1/canister/invalid_method', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        args: []
      })
    });

    expect(response.status).toBe(200);
    const result = await response.json();
    expect(result.success).toBe(true);
    expect(result.data).toHaveProperty('Err');
    expect(result.data.Err).toContain('not implemented');
  });

  it('should maintain binary payload compatibility', async () => {
    // Test that binary data is properly handled throughout the pipeline
    const pubkey = Array.from({ length: 32 }, (_, i) => i); // Deterministic test data
    const offeringData = Array.from({ length: 50 }, (_, i) => (i * 2) % 256);
    const signature = Array.from({ length: 64 }, (_, i) => (i * 3) % 256);

    const response = await worker.fetch('/api/v1/canister/provider_update_offering_anonymous', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        args: [pubkey, offeringData, signature, null]
      })
    });

    expect(response.status).toBe(200);
    const result = await response.json();
    expect(result.success).toBe(true);
    expect(result.data).toHaveProperty('Ok');

    // Verify the data was stored correctly in D1
    const pubkeyHex = pubkey.map(b => b.toString(16).padStart(2, '0')).join('');
    const testDb = (worker as any).env.TEST_DB;
    const storedOffering = await testDb.prepare(`
      SELECT offering_data FROM provider_offerings WHERE provider_pubkey = ?
    `).bind(pubkeyHex).first();

    expect(storedOffering).toBeTruthy();
    const storedData = new Uint8Array(storedOffering.offering_data);
    expect(Array.from(storedData)).toEqual(offeringData);
  });
});