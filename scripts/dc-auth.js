#!/usr/bin/env node
/**
 * dc-auth.js — Browser auth setup for development and LLM-driven testing.
 *
 * Handles account creation, browser login, and provider setup in one place.
 * After each command the browser has the seed phrase in localStorage so
 * subsequent `browser.js` calls see the authenticated state.
 *
 * Commands:
 *   create-user [username] [email]     Register new account, log in browser, snap dashboard
 *   login-user  <seed phrase words…>   Inject seed phrase into browser, snap dashboard
 *   create-provider <seed phrase…>     Create minimal offering, snap provider offerings page
 *
 * Environment:
 *   DC_API_URL         API base URL (default: https://dev-api.decent-cloud.org)
 *   DC_WEB_URL         Frontend URL for browser nav (default: http://127.0.0.1:59010)
 *   BROWSER_TIMEOUT    Navigation timeout ms (default: 20000)
 *
 * Output: JSON summary on stdout, accessibility snap on stderr prefixed with [SNAP].
 * Errors: non-zero exit with message on stderr.
 */

'use strict';

const NM      = '/code/decent-cloud/website/node_modules';
const bip39   = require(`${NM}/bip39`);
const { hmac }      = require(`${NM}/@noble/hashes/hmac`);
const { sha512 }    = require(`${NM}/@noble/hashes/sha512`);
const { ed25519ph } = require(`${NM}/@noble/curves/ed25519`);
const { chromium }  = require('/home/ubuntu/.npm-global/lib/node_modules/playwright');
const https   = require('https');
const http    = require('http');
const { randomUUID } = require('crypto');

const API_URL = process.env.DC_API_URL || 'https://dev-api.decent-cloud.org';
const WEB_URL = process.env.DC_WEB_URL || 'http://127.0.0.1:59010';
const TIMEOUT = parseInt(process.env.BROWSER_TIMEOUT || '20000', 10);
const EC      = new TextEncoder();

// ── Crypto helpers ──────────────────────────────────────────────────────────

function bytesToHex(buf) {
  return Array.from(buf).map(b => b.toString(16).padStart(2, '0')).join('');
}

/** BIP39 mnemonic → 32-byte Ed25519 seed (mirrors identityFromSeed in identity.ts) */
function secretKeyFromMnemonic(mnemonic) {
  if (!bip39.validateMnemonic(mnemonic)) throw new Error(`Invalid mnemonic: "${mnemonic}"`);
  const seed  = bip39.mnemonicToSeedSync(mnemonic, '');
  const km    = hmac(sha512, 'ed25519 seed', new Uint8Array(seed));
  return km.slice(0, 32);
}

/** Sign an API request — mirrors signRequest() in auth-api.ts */
function buildHeaders(secretKey, method, path, body = '') {
  const pubkey    = ed25519ph.getPublicKey(secretKey);
  const nonce     = randomUUID();
  const ts        = String(BigInt(Date.now()) * BigInt(1_000_000));
  const pathClean = path.split('?')[0];
  const msg       = EC.encode(ts + nonce + method + pathClean + body);
  const sig       = ed25519ph.sign(msg, secretKey, { context: EC.encode('decent-cloud') });
  return {
    'X-Public-Key':  bytesToHex(pubkey),
    'X-Signature':   bytesToHex(sig),
    'X-Timestamp':   ts,
    'X-Nonce':       nonce,
    'Content-Type':  'application/json',
  };
}

// ── HTTP helper ──────────────────────────────────────────────────────────────

function apiRequest(method, path, headers, body) {
  return new Promise((resolve, reject) => {
    const url     = new URL(API_URL + path);
    const mod     = url.protocol === 'https:' ? https : http;
    const data    = body || '';
    const reqOpts = {
      hostname: url.hostname,
      port:     url.port || (url.protocol === 'https:' ? 443 : 80),
      path:     url.pathname + url.search,
      method,
      headers:  { ...headers, 'Content-Length': Buffer.byteLength(data) },
    };
    const req = mod.request(reqOpts, res => {
      let raw = '';
      res.on('data', c => { raw += c; });
      res.on('end', () => {
        try { resolve(JSON.parse(raw)); }
        catch { reject(new Error(`Non-JSON response (${res.statusCode}): ${raw.slice(0, 200)}`)); }
      });
    });
    req.on('error', reject);
    if (data) req.write(data);
    req.end();
  });
}

// ── Browser helpers ──────────────────────────────────────────────────────────

async function openBrowser() {
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--disable-setuid-sandbox'] });
  return { browser, page: await browser.newPage() };
}

async function closeBrowser({ browser }) {
  await browser.close();
}

async function injectSeedAndNavigate(page, mnemonic, targetUrl) {
  // Step 1: Navigate to origin to get a same-origin localStorage context.
  // authStore.initialize() runs here with EMPTY localStorage (no-op).
  const origin = new URL(WEB_URL).origin;
  await page.goto(origin, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });

  // Step 2: Inject seed phrase — mirrors addSeedPhrase() in seed-storage.ts
  await page.evaluate((phrase) => {
    const stored = JSON.parse(localStorage.getItem('seed_phrases') || '[]');
    if (!stored.includes(phrase)) {
      stored.push(phrase);
      localStorage.setItem('seed_phrases', JSON.stringify(stored));
    }
  }, mnemonic);

  // Step 3: Navigate to a neutral dashboard page and wait for auth to settle.
  // Each page load re-runs authStore.initialize() (via root layout onMount).
  // We wait for the accounts API response so the identity is in the store
  // BEFORE we navigate to the target — pages call loadData() in their own
  // onMount and won't retry if identity is null at that point.
  const authDone = page.waitForResponse(
    r => r.url().includes('/api/v1/accounts'),
    { timeout: TIMEOUT }
  );
  await page.goto(`${origin}/dashboard`, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
  await authDone;
  await page.waitForTimeout(200); // let Svelte reactive state propagate

  // Step 4: Navigate to target — auth store re-initialises from localStorage
  // on each full navigation, and all pages now subscribe to auth changes and
  // call their load functions reactively, so data loads correctly.
  if (targetUrl !== `${origin}/dashboard`) {
    await page.goto(targetUrl, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
    await page.waitForLoadState('networkidle', { timeout: TIMEOUT }).catch(() => {});
    await page.waitForTimeout(300);
  }
}

async function snapPage(page) {
  try {
    const snap  = await page._snapshotForAI({ timeout: TIMEOUT });
    return snap.full.replace(/\s*\[ref=\w+\]/g, '');
  } catch {
    return '(snapshot unavailable)';
  }
}

// ── Commands ─────────────────────────────────────────────────────────────────

async function cmdCreateUser(args) {
  const suffix   = Date.now().toString(36).slice(-6);
  const username = args[0] || `testuser${suffix}`;
  const email    = args[1] || `${username}@dev.test`;
  const mnemonic = bip39.generateMnemonic();
  const sk       = secretKeyFromMnemonic(mnemonic);
  const pubkeyHex = bytesToHex(ed25519ph.getPublicKey(sk));

  const path   = '/api/v1/accounts';
  const body   = JSON.stringify({ username, email, publicKey: pubkeyHex });
  const hdrs   = buildHeaders(sk, 'POST', path, body);
  const result = await apiRequest('POST', path, hdrs, body);

  if (!result.success) throw new Error(`Registration failed: ${result.error}`);

  const { browser, page } = await openBrowser();
  try {
    await injectSeedAndNavigate(page, mnemonic, `${WEB_URL}/dashboard`);
    const snap = await snapPage(page);
    process.stderr.write(`[SNAP]\n${snap}\n`);
  } finally {
    await page.close();
    await closeBrowser({ browser });
  }

  const out = { username, email, seed: mnemonic, pubkey: pubkeyHex };
  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
}

async function cmdLoginUser(args) {
  if (args.length === 0) {
    throw new Error('Usage: login-user <seed phrase words…>');
  }
  const mnemonic = args.join(' ');
  const sk        = secretKeyFromMnemonic(mnemonic); // validates mnemonic
  const pubkeyHex = bytesToHex(ed25519ph.getPublicKey(sk));

  // Verify account exists
  const acct = await apiRequest('GET', `/api/v1/accounts?publicKey=${pubkeyHex}`, {});
  if (!acct.success || !acct.data) throw new Error(`No account for this seed phrase (pubkey: ${pubkeyHex})`);

  const { browser, page } = await openBrowser();
  try {
    await injectSeedAndNavigate(page, mnemonic, `${WEB_URL}/dashboard`);
    const snap = await snapPage(page);
    process.stderr.write(`[SNAP]\n${snap}\n`);
  } finally {
    await page.close();
    await closeBrowser({ browser });
  }

  const out = { username: acct.data.username, pubkey: pubkeyHex };
  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
}

async function cmdCreateProvider(args) {
  if (args.length === 0) {
    throw new Error('Usage: create-provider <seed phrase words…>');
  }
  const mnemonic  = args.join(' ');
  const sk        = secretKeyFromMnemonic(mnemonic);
  const pubkeyHex = bytesToHex(ed25519ph.getPublicKey(sk));

  const suffix = Date.now().toString(36).slice(-6);
  const offering = {
    pubkey:              pubkeyHex,
    offering_id:         `test-${suffix}`,
    offer_name:          `Test VPS ${suffix}`,
    description:         'Test offering created by dc-auth.js',
    currency:            'USD',
    monthly_price:       5.0,
    setup_fee:           0.0,
    visibility:          'public',
    product_type:        'vps',
    billing_interval:    'monthly',
    billing_unit:        'month',
    is_subscription:     false,
    stock_status:        'in_stock',
    datacenter_country:  'US',
    datacenter_city:     'New York',
    unmetered_bandwidth: false,
    is_draft:            true, // draft so it doesn't pollute the live marketplace
    is_example:          false,
  };

  const path   = `/api/v1/providers/${pubkeyHex}/offerings`;
  const body   = JSON.stringify(offering);
  const hdrs   = buildHeaders(sk, 'POST', path, body);
  const result = await apiRequest('POST', path, hdrs, body);

  if (!result.success) throw new Error(`Offering creation failed: ${result.error}`);

  const offeringId = result.data;
  const { browser, page } = await openBrowser();
  try {
    await injectSeedAndNavigate(page, mnemonic, `${WEB_URL}/dashboard/offerings`);
    const snap = await snapPage(page);
    process.stderr.write(`[SNAP]\n${snap}\n`);
  } finally {
    await page.close();
    await closeBrowser({ browser });
  }

  const out = { pubkey: pubkeyHex, offeringId, offeringName: offering.offer_name };
  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
}

// ── Entry point ──────────────────────────────────────────────────────────────

const [cmd, ...args] = process.argv.slice(2);

const COMMANDS = {
  'create-user':     cmdCreateUser,
  'login-user':      cmdLoginUser,
  'create-provider': cmdCreateProvider,
};

if (!cmd || !COMMANDS[cmd]) {
  process.stderr.write(
    'Usage: dc-auth.js <command> [args]\n\n' +
    'Commands:\n' +
    '  create-user [username] [email]     Register new account, log in browser\n' +
    '  login-user  <seed phrase words…>   Inject existing seed phrase, log in browser\n' +
    '  create-provider <seed phrase…>     Create minimal offering as provider\n\n' +
    'Environment:\n' +
    '  DC_API_URL       API base URL (default: https://dev-api.decent-cloud.org)\n' +
    '  DC_WEB_URL       Frontend URL (default: http://127.0.0.1:59010)\n'
  );
  process.exit(1);
}

COMMANDS[cmd](args).catch(err => {
  process.stderr.write(`Error: ${err.message}\n`);
  process.exit(1);
});
