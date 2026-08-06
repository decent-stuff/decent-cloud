// Local no-spend PoC for the Hetzner cloud-resell data path.
//
// Proves end-to-end on the slim stack (api:59011) that a REAL Hetzner resell
// offering can be: created (provider registered → cloud_account validated
// against real Hetzner → offering created with live config validation) →
// VISIBLE in the marketplace → RENTABLE (provider_online=true, which is what
// gates the marketplace Rent button).
//
// SAFETY: the ONLY real-Hetzner touchpoints here are READ-ONLY:
//   - POST /cloud-accounts validates the token (list_server_types/locations/images)
//   - GET /cloud-accounts/:id/catalog fetches the live catalog
//   - POST /providers/:pubkey/offerings validates server_type/location/image
//     against the live catalog (check_server_type_location + check_image_exists)
// No VM is created, rented, or provisioned. The script STOPS at "rentable".
//
// Uses a FRESH identity (api-cli-style) seeded DB-direct into the local DB so
// the real `hetzner-reseller` prod identity is never touched in this throwaway
// local stack. Cleanup runs in `finally` to leave the local DB clean.
//
// Usage:
//   HETZNER_API_TOKEN_DEV=... node poc/hetzner-cloud-resell-poc.mjs
// (read-write; required to create+delete test VMs. Read-only tokens 403 on
//  delete and are not injected into the agent env — see repo/AGENTS.md "Hetzner tokens".)
//
// Reusable for the sanctioned stage/prod seeding run: point API_URL + DB_URL at
// the target env and (for prod) replace `generateMnemonic` with
// `deriveIdentity(process.env.DC_PROD_RESELLER_SEED)` to act as the existing
// hetzner-reseller provider.

import { execFileSync } from 'node:child_process';
import { ed25519ph } from '../website/node_modules/@noble/curves/esm/ed25519.js';
import { hmac } from '../website/node_modules/@noble/hashes/hmac.js';
import { sha512 } from '../website/node_modules/@noble/hashes/sha512.js';
import {
  generateMnemonic,
  mnemonicToSeedSync,
} from '../website/node_modules/bip39/src/index.js';

const API_URL = process.env.API_URL || 'http://localhost:59011';
const WEB_URL = process.env.WEB_URL || 'http://localhost:59010';
const DB_URL = process.env.DATABASE_URL || 'postgres://test:test@postgres:5432/test';
// Hard-require the WRITE-capable dev token (HETZNER_API_TOKEN_DEV): read-only
// tokens 403 on create/delete and are not injected into agent sessions, so there
// is no fallback here — the script fails fast if _DEV is missing. See
// repo/AGENTS.md "Hetzner tokens".
const HETZNER_TOKEN = process.env.HETZNER_API_TOKEN_DEV;
// When set (any value), drive a headless browser to load the marketplace detail
// page and assert the Rent button is ENABLED (text "Rent this offering", not
// "Provider Offline"). Needs the warm web stack at WEB_URL.
const DO_BROWSER_CHECK = !!process.env.POC_BROWSER_CHECK;
const SIGN_CONTEXT = new TextEncoder().encode('decent-cloud');

if (!HETZNER_TOKEN) {
  console.error(
    'FAIL: set HETZNER_API_TOKEN_DEV (read-write; required to create+delete test VMs) ' +
      'for live Hetzner calls.',
  );
  process.exit(2);
}

function bytesToHex(bytes) {
  let out = '';
  for (const b of bytes) out += b.toString(16).padStart(2, '0');
  return out;
}

function randomHex(nBytes) {
  const buf = Buffer.alloc(nBytes);
  for (let i = 0; i < nBytes; i++) buf[i] = Math.floor(Math.random() * 256);
  return buf.toString('hex');
}

// Derive the Ed25519 keypair from a BIP-39 seed EXACTLY as the website does
// (HMAC-SHA512 keyed "ed25519 seed" over mnemonicToSeedSync(seed, ""), first
// 32 bytes). Matches tools/e2e-real-deployments/src/crypto.js + auth-api.ts.
function deriveIdentity(seedPhrase) {
  const seed = new Uint8Array(mnemonicToSeedSync(seedPhrase, ''));
  const keyMaterial = hmac(sha512, 'ed25519 seed', seed);
  const secretKey = keyMaterial.subarray(0, 32);
  const publicKey = ed25519ph.getPublicKey(secretKey);
  return { secretKey, publicKey, pubkeyHex: bytesToHex(publicKey) };
}

// Sign an API request: message = timestampNs ‖ nonce ‖ METHOD ‖ path(no query) ‖ body.
// Ed25519ph (SHA-512 prehash), context b"decent-cloud". Mirrors common/src/api_auth.rs.
// NOTE: the server signs `format!("/api/v1{}", uri.path())`, so the signed path
// MUST include the /api/v1 prefix. `fullPath` here is the path WITHOUT that
// prefix (e.g. "/cloud-accounts"); the prefix is prepended for both signing + URL.
function sign(identity, method, fullPath, bodyData) {
  const nonce = crypto.randomUUID();
  const timestampNs = (BigInt(Date.now()) * 1_000_000n).toString();
  let body = '';
  if (typeof bodyData === 'string') body = bodyData;
  else if (bodyData !== undefined && bodyData !== null) body = JSON.stringify(bodyData);
  const signedPath = `/api/v1${fullPath.split('?')[0]}`;
  const message = new TextEncoder().encode(
    timestampNs + nonce + method.toUpperCase() + signedPath + body,
  );
  const signature = ed25519ph.sign(message, identity.secretKey, { context: SIGN_CONTEXT });
  return {
    headers: {
      'X-Public-Key': identity.pubkeyHex,
      'X-Signature': bytesToHex(signature),
      'X-Timestamp': timestampNs,
      'X-Nonce': nonce,
      'Content-Type': 'application/json',
    },
    body,
  };
}

async function signedCall(identity, method, fullPath, bodyData) {
  const { headers, body } = sign(identity, method, fullPath, bodyData);
  const res = await fetch(`${API_URL}/api/v1${fullPath}`, {
    method,
    headers,
    body: method === 'GET' || method === 'HEAD' ? undefined : body,
  });
  const text = await res.text();
  let json = null;
  try {
    json = JSON.parse(text);
  } catch {
    /* keep null */
  }
  return { status: res.status, json, text };
}

// Seed an account DB-direct (mirrors website/tests/e2e/fixtures/seed-helpers.ts
// seedAccountDirect) so a fresh identity can make signed API calls without the
// ~10-15s UI sign-up flow.
function seedAccount(pubkeyHex, username) {
  const email = `${username}@poc.test`;
  const accountIdHex = randomHex(16);
  const keyIdHex = randomHex(16);
  // email_verified=true so the marketplace Rent button reaches its terminal
  // "Rent this offering" state (otherwise it reads "Verify email to rent" — a
  // per-viewer gate, not an offering/provider property).
  const sql = `
    INSERT INTO accounts (id, username, email, email_verified) VALUES (decode('${accountIdHex}', 'hex'), '${username}', '${email}', TRUE);
    INSERT INTO account_public_keys (id, account_id, public_key) VALUES (decode('${keyIdHex}', 'hex'), decode('${accountIdHex}', 'hex'), decode('${pubkeyHex}', 'hex'));
  `;
  execFileSync('psql', [DB_URL, '--no-psqlrc', '-v', 'ON_ERROR_STOP=1', '-c', sql], {
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  return { accountIdHex, username, email };
}

// ===================== PoC =====================
// Drive a headless browser (Playwright from website/node_modules) to the
// marketplace detail page, authenticated via the provider's seed, and read the
// Rent button. Mirrors scripts/browser.js authenticatePage: inject the seed into
// localStorage['seed_phrases'], go to /dashboard (settles the auth store), then
// navigate to the offering detail page. Returns {enabled, detail}.
async function browserRentButton(seedPhrase, offeringId) {
  const pw = (await import('../website/node_modules/playwright/index.js')).default;
  const browser = await pw.chromium.launch();
  const page = await browser.newPage();
  try {
    const origin = new URL(WEB_URL).origin;
    await page.goto(origin, { waitUntil: 'domcontentloaded' });
    await page.evaluate((phrase) => {
      const stored = JSON.parse(localStorage.getItem('seed_phrases') || '[]');
      if (!stored.includes(phrase)) stored.push(phrase);
      localStorage.setItem('seed_phrases', JSON.stringify(stored));
      localStorage.setItem('first_login_onboarding_completed', 'true');
    }, seedPhrase);
    const authDone = page.waitForResponse(
      (r) => r.url().includes('/api/v1/accounts'),
      { timeout: 15000 },
    );
    await page.goto(`${origin}/dashboard`, { waitUntil: 'domcontentloaded' });
    await authDone;
    await page.waitForTimeout(300);

    await page.goto(`${origin}/dashboard/marketplace/${offeringId}`, {
      waitUntil: 'domcontentloaded',
    });
    // Wait for the offering to actually load: the <h1> renders the offer_name
    // (only present once offering.id is defined). Poll up to ~10s.
    const offerName = '[PoC]';
    await page.waitForFunction(
      (needle) => {
        const h1 = document.querySelector('h1');
        return h1 && (h1.textContent || '').includes(needle);
      },
      offerName,
      { timeout: 10000 },
    );

    // The rent button text is one of "Rent this offering" / "Provider Offline" /
    // "Verify email to rent" (marketplace/[id]/+page.svelte:510-516).
    const btn = await page.evaluate(() => {
      const buttons = Array.from(document.querySelectorAll('button'));
      const rent = buttons.find((b) =>
        /rent this offering|provider offline|verify email to rent/i.test(b.textContent || ''),
      );
      if (!rent) return { found: false, text: '', disabled: true };
      return {
        found: true,
        text: (rent.textContent || '').trim(),
        disabled: rent.disabled || rent.getAttribute('aria-disabled') === 'true',
      };
    });
    await browser.close();
    const enabled = btn.found && !btn.disabled && /rent this offering/i.test(btn.text);
    return { enabled, detail: `found=${btn.found} disabled=${btn.disabled} text="${btn.text}"` };
  } catch (e) {
    await browser.close();
    throw e;
  }
}

const failures = [];
function check(label, cond, detail = '') {
  const tag = cond ? 'PASS' : 'FAIL';
  console.log(`  [${tag}] ${label}${detail ? ` — ${detail}` : ''}`);
  if (!cond) failures.push(label);
}

async function main() {
  // 1. Fresh identity + DB-seed the account.
  const seedPhrase = generateMnemonic(128);
  const identity = deriveIdentity(seedPhrase);
  const username = `pocresell${Date.now().toString().slice(-8)}`;
  console.log(`\n=== Hetzner cloud-resell PoC (fresh identity ${identity.pubkeyHex.slice(0, 12)}…) ===`);
  const account = seedAccount(identity.pubkeyHex, username);
  console.log(`seeded account ${username} (${account.accountIdHex.slice(0, 12)}…)`);

  let cloudAccountId = null;
  let offeringId = null;

  try {
    // 2. Register provider_profile (PUT /providers/:pubkey/onboarding).
    console.log('\n[1] register provider_profile (onboarding)');
    const onboard = await signedCall(identity, 'PUT', `/providers/${identity.pubkeyHex}/onboarding`, {
      support_email: `${username}@poc.test`,
      support_hours: '24/7',
      support_channels: 'Email',
      regions: 'Europe',
      payment_methods: 'Stripe',
      refund_policy: 'PoC provider — automated.',
      sla_guarantee: 'Best-effort (PoC).',
    });
    check('onboarding PUT success', onboard.json?.success === true, `status=${onboard.status} ${onboard.text?.slice(0, 120)}`);

    // 3. Add Hetzner cloud_account — token is validated LIVE (read-only catalog).
    //    AddCloudAccountRequest uses camelCase keys (#[serde(rename_all="camelCase")]).
    console.log('\n[2] add Hetzner cloud_account (live token validation — read-only)');
    const add = await signedCall(identity, 'POST', '/cloud-accounts', {
      backendType: 'hetzner',
      name: `poc-hetzner-${Date.now()}`,
      credentials: HETZNER_TOKEN,
    });
    check('cloud-accounts POST success', add.json?.success === true, `status=${add.status} ${add.text?.slice(0, 160)}`);
    check('token validated isValid=true', add.json?.data?.isValid === true, `isValid=${add.json?.data?.isValid}`);
    cloudAccountId = add.json?.data?.id ?? null;
    check('cloud_account id returned', typeof cloudAccountId === 'string', cloudAccountId);

    // 4. Catalog live-fetches (read-only) — proves the token really works.
    console.log('\n[3] fetch live Hetzner catalog (read-only)');
    const catalog = await signedCall(
      identity,
      'GET',
      `/cloud-accounts/${cloudAccountId}/catalog`,
    );
    check('catalog GET success', catalog.json?.success === true, `status=${catalog.status}`);
    const cat = catalog.json?.data;
    const cx23 = cat?.serverTypes?.find((s) => s.name === 'cx23');
    const nbg1 = cat?.locations?.find((l) => l.name === 'nbg1');
    const ubuntu = cat?.images?.find((i) => i.name === 'ubuntu-24.04');
    check('catalog has serverTypes', Array.isArray(cat?.serverTypes) && cat.serverTypes.length > 0);
    check('catalog has locations', Array.isArray(cat?.locations) && cat.locations.length > 0);
    check('catalog has images', Array.isArray(cat?.images) && cat.images.length > 0);
    check('cx23 in catalog', !!cx23, `priceMonthly=${cx23?.priceMonthly}`);
    check('nbg1 in catalog', !!nbg1);
    check('ubuntu-24.04 in catalog', !!ubuntu);

    // 5. Create the offering (provisioner_type=hetzner; config validated LIVE).
    //    Cheapest cx23 @ nbg1 (cx22 is retired on Hetzner; cx23 is the cheapest
    //    shared-CPU type now), EUR (Stripe-supported), public + in_stock.
    console.log('\n[4] create cloud-resell offering (live config validation — read-only)');
    const stamp = `${Date.now()}`.slice(-8);
    const offering = {
      offering_id: `poc-cx23-${stamp}`,
      offer_name: `[PoC] cx23 @ nbg1 (local no-spend)`,
      description: 'Local PoC offering — Hetzner cloud-resell data-path proof. Safe to delete.',
      currency: 'eur',
      monthly_price: 8,
      setup_fee: 0,
      visibility: 'public',
      product_type: 'compute',
      virtualization_type: 'kvm',
      billing_interval: 'monthly',
      billing_unit: 'month',
      is_subscription: true,
      subscription_interval_days: 30,
      stock_status: 'in_stock',
      processor_cores: cx23?.cores ?? 2,
      memory_amount: `${cx23?.memoryGb ?? 4} GB`,
      total_ssd_capacity: `${cx23?.diskGb ?? 40} GB`,
      unmetered_bandwidth: false,
      datacenter_country: nbg1?.country ?? 'DE',
      datacenter_city: nbg1?.city ?? 'Nuremberg',
      operating_systems: 'ubuntu-24.04',
      is_draft: false,
      provisioner_type: 'hetzner',
      provisioner_config: JSON.stringify({
        server_type: 'cx23',
        location: 'nbg1',
        image: 'ubuntu-24.04',
      }),
    };
    const create = await signedCall(
      identity,
      'POST',
      `/providers/${identity.pubkeyHex}/offerings`,
      offering,
    );
    check('offering POST success', create.json?.success === true, `status=${create.status} ${create.text?.slice(0, 160)}`);
    offeringId = create.json?.data ?? null;
    check('offering numeric id returned', typeof offeringId === 'number', `${offeringId}`);

    // 6. Marketplace visibility: GET /offerings must list it AND mark it online.
    //    provider_online===true is what keeps the Rent button ENABLED in the UI
    //    (marketplace/[id]/+page.svelte: disabled={offering.provider_online === false}).
    console.log('\n[5] marketplace visibility + rentability (provider_online)');
    const list = await signedCall(identity, 'GET', '/offerings?limit=100');
    check('offerings search success', list.json?.success === true, `status=${list.status} ${list.text?.slice(0, 120)}`);
    const found = (list.json?.data ?? []).find((o) => o.id === offeringId);
    check('offering visible in marketplace list', !!found, `count=${list.json?.data?.length}`);
    check(
      'provider_online===true (rent button ENABLED)',
      found?.provider_online === true,
      `provider_online=${found?.provider_online}`,
    );
    check(
      'provisioner_type=hetzner confirmed',
      found?.provisioner_type === 'hetzner',
      found?.provisioner_type,
    );
    check(
      'agent_pool_id null (no gateway/pool)',
      found?.agent_pool_id == null,
      `${found?.agent_pool_id}`,
    );

    // 7. Also confirm it shows in the provider's own offerings list.
    const myList = await signedCall(
      identity,
      'GET',
      `/providers/${identity.pubkeyHex}/offerings`,
    );
    const mine = (myList.json?.data ?? []).some((o) => o.id === offeringId);
    check('offering in provider own list', myList.json?.success === true && mine);

    // 8. Browser confirmation (opt-in): load the marketplace detail page as an
    //    authenticated viewer and assert the Rent button is ENABLED. The button
    //    is disabled iff provider_online === false (marketplace/[id]/+page.svelte),
    //    so "Rent this offering" (not "Provider Offline") is the rentable proof.
    if (DO_BROWSER_CHECK) {
      console.log('\n[6] browser: marketplace detail page Rent button state');
      try {
        const btn = await browserRentButton(seedPhrase, offeringId);
        check('rent button ENABLED (text "Rent this offering")', btn.enabled, btn.detail);
      } catch (e) {
        check('rent button browser check', false, e.message);
      }
    }

    console.log(`\n=== RESULT: ${failures.length === 0 ? 'PoC PASSED — data path proven (STOP before rent)' : `${failures.length} CHECK(S) FAILED`} ===`);
    if (failures.length) {
      console.log('Failed:', failures);
    }
  } finally {
    // Cleanup: leave the local DB clean. Best-effort; surface errors.
    console.log('\n[cleanup] removing offering, cloud_account, provider profile, account…');
    try {
      if (offeringId != null) {
        const r = await signedCall(
          identity,
          'DELETE',
          `/providers/${identity.pubkeyHex}/offerings/${offeringId}`,
        );
        console.log(`  offering ${offeringId} delete: ${r.json?.success ? 'ok' : r.text?.slice(0, 100)}`);
      }
    } catch (e) {
      console.log(`  offering delete error: ${e.message}`);
    }
    try {
      if (cloudAccountId) {
        const r = await signedCall(identity, 'DELETE', `/cloud-accounts/${cloudAccountId}`);
        console.log(`  cloud_account ${cloudAccountId} delete: ${r.json?.success ? 'ok' : r.text?.slice(0, 100)}`);
      }
    } catch (e) {
      console.log(`  cloud_account delete error: ${e.message}`);
    }
    try {
      // provider_profiles + provider_onboarding are not cascaded by accounts DELETE.
      const safe = username.replace(/'/g, "''");
      const sql = `
        DELETE FROM provider_onboarding WHERE provider_pubkey = decode('${identity.pubkeyHex}', 'hex');
        DELETE FROM provider_profiles WHERE pubkey = decode('${identity.pubkeyHex}', 'hex');
        DELETE FROM signature_audit WHERE account_id = (SELECT id FROM accounts WHERE username = '${safe}');
        DELETE FROM reseller_commissions_mapping WHERE referred_account_id = (SELECT id FROM accounts WHERE username = '${safe}');
        DELETE FROM accounts WHERE username = '${safe}';
      `;
      execFileSync('psql', [DB_URL, '--no-psqlrc', '-c', sql], { stdio: ['ignore', 'ignore', 'pipe'] });
      console.log('  account + provider profile removed (ok)');
    } catch (e) {
      console.log(`  account/profile cleanup error: ${e.message}`);
    }
  }

  if (failures.length) process.exit(1);
}

main().catch((e) => {
  console.error('PoC threw:', e);
  process.exit(1);
});
