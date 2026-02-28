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
 * Environment:
 *   BROWSER_TIMEOUT  Navigation timeout ms (default: 20000)
 *
 * Note: uses page._snapshotForAI() (Playwright 1.44+ internal) for snap.
 * Returns a YAML-like accessibility tree — far more useful than raw DOM.
 */

'use strict';

const { chromium } = require('/home/ubuntu/.npm-global/lib/node_modules/playwright');

const TIMEOUT = parseInt(process.env.BROWSER_TIMEOUT || '20000', 10);

const [,, cmd, url, ...rest] = process.argv;

if (!cmd || !url) {
  console.error('Usage: browser.js <snap|shot|eval|errs|html> <url> [args...]');
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
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
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
        const outPath = rest[0] || '/tmp/browser-shot.png';
        await page.screenshot({ path: outPath, fullPage: false });
        console.log(outPath);
        break;
      }

      case 'eval': {
        const result = await page.evaluate(rest.join(' '));
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
