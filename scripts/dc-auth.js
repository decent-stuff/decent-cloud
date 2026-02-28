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
const fs      = require('fs');
const { spawn }    = require('child_process');
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

/** Sign an agent API request — identical to buildHeaders() but uses X-Agent-Pubkey */
function buildAgentHeaders(secretKey, method, path, body = '') {
  const pubkey    = ed25519ph.getPublicKey(secretKey);
  const nonce     = randomUUID();
  const ts        = String(BigInt(Date.now()) * BigInt(1_000_000));
  const pathClean = path.split('?')[0];
  const msg       = EC.encode(ts + nonce + method + pathClean + body);
  const sig       = ed25519ph.sign(msg, secretKey, { context: EC.encode('decent-cloud') });
  return {
    'X-Agent-Pubkey': bytesToHex(pubkey),
    'X-Signature':    bytesToHex(sig),
    'X-Timestamp':    ts,
    'X-Nonce':        nonce,
    'Content-Type':   'application/json',
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

async function cmdSeedUxData(args) {
  // Reuse existing account if seed phrase provided, otherwise create fresh one
  let mnemonic, sk, pubkeyHex;

  if (args.length > 0) {
    mnemonic  = args.join(' ');
    sk        = secretKeyFromMnemonic(mnemonic);
    pubkeyHex = bytesToHex(ed25519ph.getPublicKey(sk));
    // Verify account exists
    const acct = await apiRequest('GET', `/api/v1/accounts?publicKey=${pubkeyHex}`, {});
    if (!acct.success || !acct.data) throw new Error(`No account for this seed phrase (pubkey: ${pubkeyHex})`);
  } else {
    const suffix   = Date.now().toString(36).slice(-6);
    const username = `uxprovider${suffix}`;
    const email    = `${username}@dev.test`;
    mnemonic  = bip39.generateMnemonic();
    sk        = secretKeyFromMnemonic(mnemonic);
    pubkeyHex = bytesToHex(ed25519ph.getPublicKey(sk));

    const regPath = '/api/v1/accounts';
    const regBody = JSON.stringify({ username, email, publicKey: pubkeyHex });
    const regHdrs = buildHeaders(sk, 'POST', regPath, regBody);
    const regResult = await apiRequest('POST', regPath, regHdrs, regBody);
    if (!regResult.success) throw new Error(`Registration failed: ${regResult.error}`);
  }

  // 1. Create agent pool
  const poolPath = `/api/v1/providers/${pubkeyHex}/pools`;
  const poolBody = JSON.stringify({ name: 'UX Test Pool', location: 'US', provisionerType: 'proxmox' });
  const poolHdrs = buildHeaders(sk, 'POST', poolPath, poolBody);
  const poolResult = await apiRequest('POST', poolPath, poolHdrs, poolBody);
  if (!poolResult.success) throw new Error(`Pool creation failed: ${poolResult.error}`);
  const poolId = poolResult.data.poolId;

  // 2. Create setup token
  const tokenPath = `/api/v1/providers/${pubkeyHex}/pools/${poolId}/setup-tokens`;
  const tokenBody = JSON.stringify({ label: 'ux-test-agent', expiresInHours: 1 });
  const tokenHdrs = buildHeaders(sk, 'POST', tokenPath, tokenBody);
  const tokenResult = await apiRequest('POST', tokenPath, tokenHdrs, tokenBody);
  if (!tokenResult.success) throw new Error(`Setup token creation failed: ${tokenResult.error}`);
  const setupToken = tokenResult.data.token;

  // 3. Generate agent keypair and register agent (unauthenticated)
  const agentMnemonic = bip39.generateMnemonic();
  const agentSk       = secretKeyFromMnemonic(agentMnemonic);
  const agentPubkeyHex = bytesToHex(ed25519ph.getPublicKey(agentSk));

  const setupPath = '/api/v1/agents/setup';
  const setupBody = JSON.stringify({ token: setupToken, agentPubkey: agentPubkeyHex });
  const setupResult = await apiRequest('POST', setupPath, { 'Content-Type': 'application/json' }, setupBody);
  if (!setupResult.success) throw new Error(`Agent setup failed: ${setupResult.error}`);

  // 4. Send heartbeat to mark agent online (uses X-Agent-Pubkey auth)
  const hbPath = `/api/v1/providers/${pubkeyHex}/heartbeat`;
  const hbBody = JSON.stringify({ activeContracts: 0, provisionerType: 'proxmox', capabilities: ['vm'] });
  const hbHdrs = buildAgentHeaders(agentSk, 'POST', hbPath, hbBody);
  const hbResult = await apiRequest('POST', hbPath, hbHdrs, hbBody);
  if (!hbResult.success) throw new Error(`Heartbeat failed: ${hbResult.error}`);

  // 5. Create 3 KVM offerings
  const offeringDefs = [
    {
      offer_name: 'Basic KVM', description: '1 vCPU / 1 GB RAM / 25 GB SSD',
      monthly_price: 3.0, datacenter_city: 'New York', datacenter_country: 'US',
      processor_cores: 1, memory_amount: '1 GB', total_ssd_capacity: '25 GB',
    },
    {
      offer_name: 'Standard KVM', description: '2 vCPU / 4 GB RAM / 50 GB SSD',
      monthly_price: 9.0, datacenter_city: 'Frankfurt', datacenter_country: 'DE',
      processor_cores: 2, memory_amount: '4 GB', total_ssd_capacity: '50 GB',
    },
    {
      offer_name: 'Pro KVM', description: '4 vCPU / 8 GB RAM / 100 GB SSD',
      monthly_price: 18.0, datacenter_city: 'Amsterdam', datacenter_country: 'NL',
      processor_cores: 4, memory_amount: '8 GB', total_ssd_capacity: '100 GB',
    },
  ];

  const offeringIds = [];
  const offeringsPath = `/api/v1/providers/${pubkeyHex}/offerings`;

  for (const def of offeringDefs) {
    const suffix = Date.now().toString(36).slice(-6);
    const offering = {
      pubkey:               pubkeyHex,
      offering_id:          `ux-${def.offer_name.toLowerCase().replace(/\s+/g, '-')}-${suffix}`,
      offer_name:           def.offer_name,
      description:          def.description,
      currency:             'USD',
      monthly_price:        def.monthly_price,
      setup_fee:            0.0,
      visibility:           'public',
      product_type:         'vps',
      virtualization_type:  'KVM',
      billing_interval:     'monthly',
      billing_unit:         'month',
      is_subscription:      false,
      stock_status:         'in_stock',
      is_draft:             false,
      is_example:           false,
      datacenter_country:   def.datacenter_country,
      datacenter_city:      def.datacenter_city,
      operating_systems:    'Ubuntu 22.04,Debian 12,Rocky Linux 9',
      min_contract_hours:   24,
      max_contract_hours:   8760,
      unmetered_bandwidth:  false,
      processor_amount:     1,
      processor_cores:      def.processor_cores,
      memory_amount:        def.memory_amount,
      ssd_amount:           1,
      total_ssd_capacity:   def.total_ssd_capacity,
      agent_pool_id:        poolId,
    };
    const offerBody = JSON.stringify(offering);
    const offerHdrs = buildHeaders(sk, 'POST', offeringsPath, offerBody);
    const offerResult = await apiRequest('POST', offeringsPath, offerHdrs, offerBody);
    if (!offerResult.success) throw new Error(`Offering "${def.offer_name}" creation failed: ${offerResult.error}`);
    offeringIds.push(offerResult.data);
  }

  // 6. Spawn background keepalive daemon to hold agent online past the 5-min window
  const pidFile = `/tmp/dc-keepalive-${pubkeyHex.slice(0, 8)}.pid`;
  const keepalive = spawn(process.execPath, [__filename, '_keepalive', ...agentMnemonic.split(' '), pubkeyHex], {
    detached: true,
    stdio: 'ignore',
  });
  keepalive.unref();
  fs.writeFileSync(pidFile, String(keepalive.pid));
  process.stderr.write(`[KEEPALIVE] Heartbeat daemon started (PID ${keepalive.pid}). Stop: kill $(cat ${pidFile})\n`);

  // 7. Open browser to marketplace (authenticated)
  const { browser, page } = await openBrowser();
  try {
    await injectSeedAndNavigate(page, mnemonic, `${WEB_URL}/dashboard/marketplace`);
    const snap = await snapPage(page);
    process.stderr.write(`[SNAP]\n${snap}\n`);
  } finally {
    await page.close();
    await closeBrowser({ browser });
  }

  const out = { seed: mnemonic, agentSeed: agentMnemonic, pubkey: pubkeyHex, poolId, offeringIds };
  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
}

// Internal: heartbeat keepalive daemon — spawned by seed-ux-data, runs until killed.
// Args: <agent-seed-12-words...> <provider-pubkey-hex>
async function cmdKeepalive(args) {
  if (args.length < 13) throw new Error('_keepalive requires 12 seed words + provider pubkey');
  const providerPubkeyHex = args[args.length - 1];
  const agentMnemonic = args.slice(0, -1).join(' ');
  const agentSk = secretKeyFromMnemonic(agentMnemonic);

  // Send heartbeats every 3 minutes (well within the 5-minute online window)
  while (true) {
    try {
      const hbPath = `/api/v1/providers/${providerPubkeyHex}/heartbeat`;
      const hbBody = JSON.stringify({ activeContracts: 0, provisionerType: 'proxmox', capabilities: ['vm'] });
      const hbHdrs = buildAgentHeaders(agentSk, 'POST', hbPath, hbBody);
      await apiRequest('POST', hbPath, hbHdrs, hbBody);
    } catch { /* best-effort: transient failures don't crash the daemon */ }
    await new Promise(r => setTimeout(r, 3 * 60 * 1000));
  }
}

// ── Entry point ──────────────────────────────────────────────────────────────

const [cmd, ...args] = process.argv.slice(2);

const COMMANDS = {
  'create-user':     cmdCreateUser,
  'login-user':      cmdLoginUser,
  'create-provider': cmdCreateProvider,
  'seed-ux-data':    cmdSeedUxData,
  '_keepalive':      cmdKeepalive, // internal: spawned by seed-ux-data
};

if (!cmd || !COMMANDS[cmd]) {
  process.stderr.write(
    'Usage: dc-auth.js <command> [args]\n\n' +
    'Commands:\n' +
    '  create-user [username] [email]     Register new account, log in browser\n' +
    '  login-user  <seed phrase words…>   Inject existing seed phrase, log in browser\n' +
    '  create-provider <seed phrase…>     Create minimal offering as provider\n' +
    '  seed-ux-data [seed phrase words…]  Bootstrap online UX test provider with 3 KVM offerings\n\n' +
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
