// REAL-VM probe: rent a Hetzner cloud-resell offering → measure provisioning →
// cancel → measure cleanup. Captures per-step timing + Hetzner-side state.
//
// SAFETY (MINIMIZE-CLOUD-SPENDING):
//   - cx23 only (cheapest), nbg1, ubuntu-24.04
//   - VM is DELETED in finally even on timeout/error
//   - wrapped externally with `timeout 600 node ...`
//
// Path: provider(self) → cloud_account(live token) → offering → self-rent
// (payment_method=test, no Stripe) → auto-accept (auto_accept_rentals=true) →
// try_trigger_cloud_provisioning → cloud_resource(provisioning) → provisioning
// service creates VM → contract active. Then cancel → termination service deletes VM.
//
// Usage:
//   HETZNER_API_TOKEN_DEV=... timeout 600 node poc/hetzner-provision-probe.mjs

import { execFileSync } from 'node:child_process';
import { ed25519ph } from '../website/node_modules/@noble/curves/esm/ed25519.js';
import { hmac } from '../website/node_modules/@noble/hashes/hmac.js';
import { sha512 } from '../website/node_modules/@noble/hashes/sha512.js';
import { generateMnemonic, mnemonicToSeedSync } from '../website/node_modules/bip39/src/index.js';

const API_URL = process.env.API_URL || 'http://localhost:59011';
const DB_URL = process.env.DATABASE_URL || 'postgres://test:test@postgres:5432/test';
const HETZNER_TOKEN = process.env.HETZNER_API_TOKEN_DEV;
const SSH_PUBKEY = process.env.SSH_PUBKEY || execFileSync('cat', ['/tmp/dc-probe-key/id_ed25519.pub']).toString().trim();
const SIGN_CONTEXT = new TextEncoder().encode('decent-cloud');

if (!HETZNER_TOKEN) {
	console.error('FAIL: HETZNER_API_TOKEN_DEV env var is required (the read-write dev token agents use for ALL Hetzner dev/experimentation).');
	process.exit(2);
}

const T = () => Date.now(); // epoch ms
const ts = (t) => new Date(t).toISOString();
const since = (t0) => `${((T() - t0) / 1000).toFixed(1)}s`;

function bytesToHex(b) { let o = ''; for (const x of b) o += x.toString(16).padStart(2, '0'); return o; }
function randomHex(n) { const b = Buffer.alloc(n); for (let i = 0; i < n; i++) b[i] = Math.floor(Math.random() * 256); return b.toString('hex'); }

function deriveIdentity(seedPhrase) {
  const seed = new Uint8Array(mnemonicToSeedSync(seedPhrase, ''));
  const km = hmac(sha512, 'ed25519 seed', seed);
  const sk = km.subarray(0, 32);
  const pk = ed25519ph.getPublicKey(sk);
  return { secretKey: sk, publicKey: pk, pubkeyHex: bytesToHex(pk) };
}

function sign(id, method, fullPath, bodyData) {
  const nonce = crypto.randomUUID();
  const tNs = (BigInt(Date.now()) * 1_000_000n).toString();
  let body = '';
  if (typeof bodyData === 'string') body = bodyData;
  else if (bodyData != null) body = JSON.stringify(bodyData);
  const signedPath = `/api/v1${fullPath.split('?')[0]}`;
  const msg = new TextEncoder().encode(tNs + nonce + method.toUpperCase() + signedPath + body);
  const sig = ed25519ph.sign(msg, id.secretKey, { context: SIGN_CONTEXT });
  return { headers: { 'X-Public-Key': id.pubkeyHex, 'X-Signature': bytesToHex(sig), 'X-Timestamp': tNs, 'X-Nonce': nonce, 'Content-Type': 'application/json' }, body };
}

async function call(id, method, fullPath, bodyData) {
  const { headers, body } = sign(id, method, fullPath, bodyData);
  const t0 = T();
  const res = await fetch(`${API_URL}/api/v1${fullPath}`, { method, headers, body: method === 'GET' ? undefined : body });
  const text = await res.text();
  let json = null; try { json = JSON.parse(text); } catch { /* */ }
  return { status: res.status, json, text, ms: T() - t0 };
}

function seedAccount(pubkeyHex, username) {
  const email = `${username}@poc.test`;
  const aid = randomHex(16), kid = randomHex(16);
  const sql = `
    INSERT INTO accounts (id, username, email, email_verified) VALUES (decode('${aid}','hex'), '${username}', '${email}', TRUE);
    INSERT INTO account_public_keys (id, account_id, public_key) VALUES (decode('${kid}','hex'), decode('${aid}','hex'), decode('${pubkeyHex}','hex'));
  `;
  execFileSync('psql', [DB_URL, '--no-psqlrc', '-v', 'ON_ERROR_STOP=1', '-c', sql], { stdio: ['ignore', 'ignore', 'pipe'] });
  return { aid, username, email };
}

// Hetzner direct (read-only list + delete fallback) using the SAME token the
// cloud_account stores (== HETZNER_TOKEN here, so delete should succeed).
async function hz(method, path) {
  const res = await fetch(`https://api.hetzner.cloud/v1${path}`, {
    method,
    headers: { Authorization: `Bearer ${HETZNER_TOKEN}`, 'Content-Type': 'application/json' },
  });
  const text = await res.text();
  let json = null; try { json = JSON.parse(text); } catch { /* */ }
  return { status: res.status, json, text };
}

async function hzFindServerByName(namePrefix) {
  const { json } = await hz('GET', '/servers');
  return (json?.servers || []).find((s) => s.name.startsWith(namePrefix)) || null;
}

// Poll contract status + cloud_resource status + Hetzner server until predicate or timeout.
async function poll(label, contractIdHex, namePrefix, deadlineMs, pred) {
  const start = T();
  let last = null;
  while (T() < deadlineMs) {
    const c = await call(identity, 'GET', `/contracts/${contractIdHex}`);
    const cStatus = c.json?.data?.status;
    // cloud_resource status via DB-direct (the real state machine)
    let crStatus = null, crExternal = null;
    try {
      const out = execFileSync('psql', [DB_URL, '--no-psqlrc', '-At', '-c',
        `SELECT status, external_id FROM cloud_resources WHERE contract_id = decode('${contractIdHex}','hex') LIMIT 1`],
        { encoding: 'utf8' });
      const [s, e] = out.trim().split('|');
      crStatus = s; crExternal = e;
    } catch { /* */ }
    const srv = await hzFindServerByName(namePrefix);
    const snap = { cStatus, crStatus, crExternal, hzStatus: srv?.status || null, hzId: srv?.id || null };
    const key = `${cStatus}|${crStatus}|${snap.hzStatus}`;
    if (key !== last) {
      console.log(`  [poll ${label}] +${since(start)} contract=${cStatus} cloud_resource=${crStatus} hz_server=${snap.hzStatus}(id=${snap.hzId}) @ ${ts(T())}`);
      last = key;
    }
    if (pred(snap)) return snap;
    await new Promise((r) => setTimeout(r, 2000));
  }
  console.log(`  [poll ${label}] TIMEOUT after ${since(start)}`);
  return null;
}

let identity;

async function main() {
  const seedPhrase = generateMnemonic(128);
  identity = deriveIdentity(seedPhrase);
  const username = `probe${Date.now().toString().slice(-8)}`;
  console.log(`\n=== Hetzner provision probe (identity ${identity.pubkeyHex.slice(0, 12)}…) ===`);
  console.log(`ssh_pubkey: ${SSH_PUBKEY.slice(0, 40)}…`);
  const account = seedAccount(identity.pubkeyHex, username);
  console.log(`seeded account ${username}`);

  let cloudAccountId = null, offeringId = null, contractIdHex = null;
  const timings = {};

  try {
    // 1. provider onboarding
    let r = await call(identity, 'PUT', `/providers/${identity.pubkeyHex}/onboarding`, {
      support_email: `${username}@poc.test`, support_hours: '24/7', support_channels: 'Email',
      regions: 'Europe', payment_methods: 'Stripe', refund_policy: 'probe', sla_guarantee: 'probe',
    });
    if (r.json?.success !== true) throw new Error(`onboarding failed: ${r.status} ${r.text?.slice(0, 160)}`);
    console.log(`[1] onboarding ok (${r.ms}ms)`);

    // 2. cloud_account (live token validation)
    timings.tCloudAcctStart = T();
    r = await call(identity, 'POST', '/cloud-accounts', { backendType: 'hetzner', name: `probe-${Date.now()}`, credentials: HETZNER_TOKEN });
    if (r.json?.success !== true || r.json?.data?.isValid !== true) throw new Error(`cloud-account failed: ${r.status} ${r.text?.slice(0, 200)}`);
    cloudAccountId = r.json.data.id;
    timings.tCloudAcctEnd = T();
    console.log(`[2] cloud_account ${cloudAccountId} validated isValid=true (${r.ms}ms, ${since(timings.tCloudAcctStart)})`);

    // 3. offering cx23/nbg1/ubuntu-24.04 (NO post_provision_script → isolates status-sync bug)
    const stamp = Date.now().toString().slice(-8);
    r = await call(identity, 'POST', `/providers/${identity.pubkeyHex}/offerings`, {
      offering_id: `probe-cx23-${stamp}`, offer_name: '[probe] cx23 nbg1', description: 'probe',
      currency: 'eur', monthly_price: 8, setup_fee: 0, visibility: 'public', product_type: 'compute',
      virtualization_type: 'kvm', billing_interval: 'monthly', billing_unit: 'month', is_subscription: true,
      subscription_interval_days: 30, stock_status: 'in_stock', processor_cores: 2, memory_amount: '4 GB',
      total_ssd_capacity: '40 GB', unmetered_bandwidth: false, datacenter_country: 'DE', datacenter_city: 'Nuremberg',
      operating_systems: 'ubuntu-24.04', is_draft: false, provisioner_type: 'hetzner',
      provisioner_config: JSON.stringify({ server_type: 'cx23', location: 'nbg1', image: 'ubuntu-24.04' }),
    });
    if (r.json?.success !== true) throw new Error(`offering failed: ${r.status} ${r.text?.slice(0, 200)}`);
    offeringId = r.json.data;
    console.log(`[3] offering ${offeringId} created (${r.ms}ms)`);

    // 4. enable auto_accept_rentals (DB-direct) so self-rent auto-accepts + triggers cloud provisioning
    execFileSync('psql', [DB_URL, '--no-psqlrc', '-At', '-c',
      `UPDATE provider_profiles SET auto_accept_rentals = TRUE WHERE pubkey = decode('${identity.pubkeyHex}','hex')`],
      { stdio: ['ignore', 'ignore', 'pipe'] });
    console.log(`[4] auto_accept_rentals=TRUE set`);

    // 5. SELF-RENT (payment_method=test → no Stripe → auto-accept path)
    timings.tRentStart = T();
    r = await call(identity, 'POST', '/contracts', {
      offering_db_id: offeringId, ssh_pubkey: SSH_PUBKEY, payment_method: 'test', duration_hours: 1,
    });
    timings.tRentEnd = T();
    if (r.json?.success !== true) throw new Error(`rent failed: ${r.status} ${r.text?.slice(0, 300)}`);
    contractIdHex = r.json.data.contractId || r.json.data.contract_id || r.json.data;
    if (typeof contractIdHex !== 'string') contractIdHex = String(contractIdHex);
    console.log(`[5] rent ok contract=${contractIdHex} (${r.ms}ms) @ ${ts(timings.tRentStart)}`);
    timings.tRentReturn = timings.tRentEnd;

    const namePrefix = `dc-recipe-${contractIdHex.slice(0, 12)}`;
    console.log(`    expected VM name prefix: ${namePrefix}`);

    // 6. PROVISIONING: poll until contract=active (or hz server running + 60s grace)
    console.log('\n[6] === PROVISIONING PHASE ===');
    const provDeadline = T() + 360_000; // 6 min hard cap for provisioning
    const provSnap = await poll('prov', contractIdHex, namePrefix, provDeadline, (s) =>
      s.cStatus === 'active' || s.cStatus === 'cancelled' || s.cStatus === 'failed');
    timings.tProvEnd = T();
    if (provSnap?.cStatus === 'active') {
      console.log(`>>> CONTRACT REACHED ACTIVE at +${since(timings.tRentStart)} (prov phase ${since(timings.tRentStart)} total)`);
    } else {
      console.log(`>>> CONTRACT DID NOT REACH ACTIVE. final=${provSnap?.cStatus} (STUCK-ACCEPTED BUG likely) — waiting 60s grace to observe VM state`);
      // Observe: is the VM running while contract stays non-active? That's the bug signature.
      await new Promise((r) => setTimeout(r, 60_000));
    }

    // Record final provisioning snapshot
    const fprov = await (async () => { const c = await call(identity,'GET',`/contracts/${contractIdHex}`); const srv = await hzFindServerByName(namePrefix); return { c: c.json?.data?.status, srv: srv ? {id:srv.id,status:srv.status,ip:srv.public_net?.ipv4?.ip,created:srv.created} : null }; })();
    console.log(`    FINAL prov: contract=${fprov.c} hz_server=${JSON.stringify(fprov.srv)}`);
    timings.finalProvContract = fprov.c; timings.finalProvServer = fprov.srv;

    // 7. CANCEL + measure cleanup
    console.log('\n[7] === CANCEL + CLEANUP PHASE ===');
    timings.tCancelStart = T();
    r = await call(identity, 'PUT', `/contracts/${contractIdHex}/cancel`, { memo: 'probe cleanup' });
    timings.tCancelEnd = T();
    console.log(`[7] cancel returned status=${r.status} success=${r.json?.success} (${r.ms}ms) @ ${ts(timings.tCancelStart)}`);
    if (r.json?.success !== true) console.log(`    cancel body: ${r.text?.slice(0, 200)}`);

    // Poll until VM is GONE from Hetzner AND cloud_resource=deleted (cleanup complete)
    const cleanDeadline = T() + 420_000; // 7 min cap for cleanup
    const cleanSnap = await poll('clean', contractIdHex, namePrefix, cleanDeadline, (s) =>
      s.hzStatus === null && (s.crStatus === 'deleted' || s.crStatus === 'failed'));
    timings.tCleanEnd = T();
    if (cleanSnap && cleanSnap.hzStatus === null) {
      console.log(`>>> VM GONE + cloud_resource=${cleanSnap.crStatus} at +${since(timings.tCancelStart)} (cleanup ${since(timings.tCancelStart)})`);
    } else {
      console.log(`>>> CLEANUP INCOMPLETE after ${since(timings.tCancelStart)}: hz=${cleanSnap?.hzStatus} cr=${cleanSnap?.crStatus}`);
    }
    timings.cleanupTotalSec = ((T() - timings.tCancelStart) / 1000).toFixed(1);

    // Summary
    console.log('\n=== TIMING SUMMARY ===');
    console.log(JSON.stringify({
      rent_return_ms: timings.tRentEnd - timings.tRentStart,
      prov_total_s: timings.tProvEnd ? ((timings.tProvEnd - timings.tRentStart) / 1000).toFixed(1) : null,
      contract_final_after_prov: timings.finalProvContract,
      cancel_return_ms: timings.tCancelEnd - timings.tCancelStart,
      cleanup_total_s: timings.cleanupTotalSec,
      hz_server_final: timings.finalProvServer,
    }, null, 2));
  } finally {
    console.log('\n[finally] cleanup offering/cloud_account/account + any stranded VM...');
    // Delete the VM directly on Hetzner if it still exists (defensive; same token → should work)
    try {
      if (contractIdHex) {
        const srv = await hzFindServerByName(`dc-recipe-${contractIdHex.slice(0, 12)}`);
        if (srv) {
          console.log(`  stranded VM id=${srv.id} name=${srv.name} — deleting directly`);
          const d = await hz('DELETE', `/servers/${srv.id}`);
          console.log(`  direct VM delete: ${d.status}`);
        }
      }
    } catch (e) { console.log(`  VM cleanup error: ${e.message}`); }
    try { if (offeringId != null) { const r = await call(identity,'DELETE',`/providers/${identity.pubkeyHex}/offerings/${offeringId}`); console.log(`  offering delete: ${r.json?.success ? 'ok' : r.text?.slice(0,80)}`); } } catch (e) {}
    try { if (cloudAccountId) { const r = await call(identity,'DELETE',`/cloud-accounts/${cloudAccountId}`); console.log(`  cloud_account delete: ${r.json?.success ? 'ok' : r.text?.slice(0,80)}`); } } catch (e) {}
    try {
      const safe = username.replace(/'/g, "''");
      execFileSync('psql', [DB_URL, '--no-psqlrc', '-c',
        `DELETE FROM provider_onboarding WHERE provider_pubkey = decode('${identity.pubkeyHex}','hex'); DELETE FROM provider_profiles WHERE pubkey = decode('${identity.pubkeyHex}','hex'); DELETE FROM cloud_resources WHERE contract_id IN (SELECT contract_id FROM contract_sign_requests WHERE requester_pubkey = decode('${identity.pubkeyHex}','hex')); DELETE FROM signature_audit WHERE account_id = (SELECT id FROM accounts WHERE username='${safe}'); DELETE FROM contract_sign_requests WHERE requester_pubkey = decode('${identity.pubkeyHex}','hex'); DELETE FROM accounts WHERE username='${safe}';`],
        { stdio: ['ignore', 'ignore', 'pipe'] });
      console.log('  account/profile/contracts removed (ok)');
    } catch (e) { console.log(`  account cleanup error: ${e.message.slice(0,160)}`); }
  }
}

main().catch((e) => { console.error('probe threw:', e); process.exit(1); });
