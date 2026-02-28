#!/usr/bin/env node
/**
 * Browser helper — uses Playwright's local headless Chromium.
 *
 * Every invocation opens a NEW page, performs the operation, and closes it.
 * Works from any context: main session, subagents, CI.
 *
 * Usage:
 *   node scripts/browser.js snap  <url>             # aria/accessibility tree
 *   node scripts/browser.js shot  <url> [out.png]   # screenshot → /tmp/browser-shot.png
 *   node scripts/browser.js eval  <url> <js-expr>   # evaluate JS, print JSON result
 *   node scripts/browser.js errs  <url>             # console errors/warnings only
 *   node scripts/browser.js html  <url>             # raw page HTML (truncated to 50k)
 *
 * Options:
 *   --seed <phrase>    Inject seed phrase into localStorage before navigating
 *
 * Environment:
 *   BROWSER_TIMEOUT  Navigation timeout ms (default: 20000)
 *   DC_WEB_URL       Frontend URL (default: http://127.0.0.1:59010)
 *
 * Note: uses page._snapshotForAI() (Playwright 1.44+ internal) for snap.
 * Returns a YAML-like accessibility tree — far more useful than raw DOM.
 */

'use strict';

const { chromium } = require('/home/ubuntu/.npm-global/lib/node_modules/playwright');

const TIMEOUT = parseInt(process.env.BROWSER_TIMEOUT || '20000', 10);
const WEB_URL = process.env.DC_WEB_URL || 'http://127.0.0.1:59010';

const [,, cmd, url, ...rest] = process.argv;

// Parse optional --seed argument
// Usage: browser.js <cmd> <url> [args...] --seed <phrase>
// The --seed flag and seed phrase MUST be the last arguments
let seedPhrase = null;
let filteredRest = rest;
const seedIndex = rest.indexOf('--seed');
if (seedIndex !== -1) {
  seedPhrase = rest.slice(seedIndex + 1).join(' ');
  filteredRest = rest.slice(0, seedIndex);
}

if (!cmd || !url) {
  console.error('Usage: browser.js <snap|shot|eval|errs|html> <url> [args...]');
  console.error('Options: --seed <phrase>  Inject seed phrase for authenticated testing');
  process.exit(1);
}

async function main() {
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--disable-setuid-sandbox'] });
  const page = await browser.newPage();

  const consoleLogs = [];
  page.on('console', msg => {
    const type = msg.type();
    if (['error', 'warning', 'warn'].includes(type)) {
      consoleLogs.push(`[${type.toUpperCase()}] ${msg.text()}`);
    }
  });
  page.on('pageerror', err => consoleLogs.push(`[PAGEERROR] ${err.message}`));

  try {
    // If seed phrase provided, inject it into localStorage before navigating
    if (seedPhrase) {
      const origin = new URL(url.startsWith('http') ? url : `${WEB_URL}${url}`).origin;
      // Step 1: Navigate to origin to get same-origin localStorage context
      await page.goto(origin, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });

      // Step 2: Inject seed phrase into localStorage['seed_phrases']
      await page.evaluate((phrase) => {
        const stored = JSON.parse(localStorage.getItem('seed_phrases') || '[]');
        if (!stored.includes(phrase)) {
          stored.push(phrase);
          localStorage.setItem('seed_phrases', JSON.stringify(stored));
        }
      }, seedPhrase);

      // Step 3: Navigate to dashboard and wait for auth to settle.
      // Pages call loadData() in onMount; if identity is null at that point they skip it.
      // Waiting for /api/v1/accounts response ensures the auth store is populated
      // before we navigate to the target page.
      const authDone = page.waitForResponse(
        r => r.url().includes('/api/v1/accounts'),
        { timeout: TIMEOUT }
      );
      await page.goto(`${origin}/dashboard`, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
      await authDone;
      await page.waitForTimeout(200); // let Svelte reactive state propagate

      // Step 4: Navigate to the actual target URL
      const targetUrl = url.startsWith('http') ? url : `${origin}${url}`;
      if (targetUrl !== `${origin}/dashboard`) {
        await page.goto(targetUrl, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
      }
    } else {
      await page.goto(url, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
    }
    // Wait for JS-rendered content (SvelteKit hydrates after domcontentloaded)
    await page.waitForLoadState('networkidle', { timeout: TIMEOUT }).catch(e => {
      process.stderr.write(`[WARN] networkidle timeout: ${e.message}\n`);
    });

    switch (cmd) {
      case 'snap': {
        // Accessibility tree snapshot — structured YAML, far more useful than raw DOM.
        // _snapshotForAI() is Playwright's internal method (public as ariaSnapshot in later builds).
        const snapshot = await page._snapshotForAI({ timeout: TIMEOUT });
        // Strip [ref=...] internal Playwright references — they add noise without value
        const clean = snapshot.full.replace(/\s*\[ref=\w+\]/g, '');
        process.stdout.write(clean + '\n');
        if (consoleLogs.length) {
          process.stdout.write('\n--- Console ---\n' + consoleLogs.join('\n') + '\n');
        }
        break;
      }

      case 'shot': {
        const outPath = filteredRest[0] || '/tmp/browser-shot.png';
        await page.screenshot({ path: outPath, fullPage: false });
        console.log(outPath);
        break;
      }

      case 'eval': {
        const result = await page.evaluate(filteredRest.join(' '));
        console.log(JSON.stringify(result, null, 2));
        break;
      }

      case 'errs': {
        if (consoleLogs.length === 0) {
          console.log('(no console errors or warnings)');
        } else {
          consoleLogs.forEach(l => console.log(l));
        }
        break;
      }

      case 'html': {
        const html = await page.content();
        process.stdout.write(html.slice(0, 50000) + '\n');
        break;
      }

      default:
        console.error(`Unknown command: ${cmd}. Use: snap, shot, eval, errs, html`);
        process.exit(1);
    }
  } finally {
    await page.close();
    await browser.close();
  }
}

main().catch(e => {
  console.error('ERROR:', e.message);
  process.exit(1);
});
