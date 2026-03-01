#!/usr/bin/env node
/**
 * Browser helper — uses Playwright's local headless browser (Chromium by default).
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

const { chromium, firefox } = require('/home/ubuntu/.npm-global/lib/node_modules/playwright');
const fs = require('fs');

const TIMEOUT = parseInt(process.env.BROWSER_TIMEOUT || '20000', 10);
const WEB_URL = process.env.DC_WEB_URL || 'http://127.0.0.1:59010';
const MOBILE_VIEWPORT = { width: 375, height: 812 };
const BROWSER_ENGINE = (process.env.BROWSER_ENGINE || 'chromium').toLowerCase();

const TOUR_ROUTES = [
  { path: '/', name: 'Landing', public: true },
  { path: '/dashboard/marketplace', name: 'Marketplace' },
  { path: '/dashboard/rentals', name: 'My Rentals' },
  { path: '/dashboard/offerings', name: 'Provider Offerings' },
  { path: '/dashboard/provider/requests', name: 'Provider Requests' },
  { path: '/dashboard/provider/analytics', name: 'Provider Analytics' },
  { path: '/dashboard/provider/agents', name: 'Provider Agents' },
  { path: '/dashboard/account', name: 'Account Settings' },
];

const [,, cmd, ...remainingArgs] = process.argv;

let url = null;
let seedPhrase = null;
let viewport = null;
let filteredRest = [];

if (cmd === 'tour') {
  const seedIndex = remainingArgs.indexOf('--seed');
  if (seedIndex !== -1) {
    seedPhrase = remainingArgs.slice(seedIndex + 1).join(' ');
  }
  const viewportIndex = remainingArgs.indexOf('--viewport');
  if (viewportIndex !== -1 && remainingArgs[viewportIndex + 1] === 'mobile') {
    viewport = MOBILE_VIEWPORT;
  }
} else {
  url = remainingArgs[0];
  const rest = remainingArgs.slice(1);

  const seedIndex = rest.indexOf('--seed');
  if (seedIndex !== -1) {
    seedPhrase = rest.slice(seedIndex + 1).join(' ');
    filteredRest = rest.slice(0, seedIndex);
  } else {
    filteredRest = rest;
  }

  const viewportIndex = filteredRest.indexOf('--viewport');
  if (viewportIndex !== -1) {
    const viewportValue = filteredRest[viewportIndex + 1];
    if (viewportValue === 'mobile') {
      viewport = MOBILE_VIEWPORT;
    }
    filteredRest = filteredRest.filter((_, i) => i !== viewportIndex && i !== viewportIndex + 1);
  }
}

if (!cmd || (cmd !== 'tour' && !url)) {
  console.error('Usage: browser.js <snap|shot|eval|errs|html|tour> <url> [args...]');
  console.error('       browser.js tour --seed <phrase> [--viewport mobile]');
  console.error('Options:');
  console.error('  --seed <phrase>    Inject seed phrase for authenticated testing');
  console.error('  --viewport mobile  Use 375×812 (iPhone X) viewport');
  process.exit(1);
}

if (cmd === 'tour' && !seedPhrase) {
  console.error('Error: tour command requires --seed <phrase>');
  process.exit(1);
}

async function main() {
  const browserType = selectBrowserType(BROWSER_ENGINE);

  if (cmd === 'tour') {
    await runTour(browserType);
    return;
  }

  const browser = await launchBrowser(browserType);
  const page = viewport
    ? await browser.newPage({ viewport })
    : await browser.newPage();

  const consoleLogs = [];
  page.on('console', msg => {
    const type = msg.type();
    if (['error', 'warning', 'warn'].includes(type)) {
      consoleLogs.push(`[${type.toUpperCase()}] ${msg.text()}`);
    }
  });
  page.on('pageerror', err => consoleLogs.push(`[PAGEERROR] ${err.message}`));

  try {
    if (seedPhrase) {
      const origin = new URL(url.startsWith('http') ? url : `${WEB_URL}${url}`).origin;
      await page.goto(origin, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });

      await page.evaluate((phrase) => {
        const stored = JSON.parse(localStorage.getItem('seed_phrases') || '[]');
        if (!stored.includes(phrase)) {
          stored.push(phrase);
          localStorage.setItem('seed_phrases', JSON.stringify(stored));
        }
      }, seedPhrase);

      const authDone = page.waitForResponse(
        r => r.url().includes('/api/v1/accounts'),
        { timeout: TIMEOUT }
      );
      await page.goto(`${origin}/dashboard`, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
      await authDone;
      await page.waitForTimeout(200);

      const targetUrl = url.startsWith('http') ? url : `${origin}${url}`;
      if (targetUrl !== `${origin}/dashboard`) {
        await page.goto(targetUrl, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
      }
    } else {
      await page.goto(url, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
    }
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
        console.error(`Unknown command: ${cmd}. Use: snap, shot, eval, errs, html, tour`);
        process.exit(1);
    }
  } finally {
    await page.close();
    await browser.close();
  }
}

async function runTour(browserType) {
  const browser = await launchBrowser(browserType);
  const page = viewport
    ? await browser.newPage({ viewport })
    : await browser.newPage();

  const origin = new URL(WEB_URL).origin;
  const report = { timestamp: new Date().toISOString(), seed: seedPhrase, viewport, pages: [] };

  try {
    for (const route of TOUR_ROUTES) {
      const pageReport = { path: route.path, name: route.name, url: `${origin}${route.path}` };
      process.stderr.write(`[TOUR] Visiting ${route.name} (${route.path})...\n`);

      const pageConsole = [];
      const handler = msg => { if (['error', 'warning', 'warn'].includes(msg.type())) pageConsole.push(`[${msg.type().toUpperCase()}] ${msg.text()}`); };
      page.on('console', handler);

      try {
        if (route.public && !seedPhrase) {
          await page.goto(`${origin}${route.path}`, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
        } else if (route.public) {
          await page.goto(`${origin}${route.path}`, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
        } else {
          if (seedPhrase && !report._authenticated) {
            await page.goto(origin, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
            await page.evaluate((phrase) => {
              const stored = JSON.parse(localStorage.getItem('seed_phrases') || '[]');
              if (!stored.includes(phrase)) {
                stored.push(phrase);
                localStorage.setItem('seed_phrases', JSON.stringify(stored));
              }
            }, seedPhrase);
            const authDone = page.waitForResponse(r => r.url().includes('/api/v1/accounts'), { timeout: TIMEOUT });
            await page.goto(`${origin}/dashboard`, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
            await authDone;
            await page.waitForTimeout(200);
            report._authenticated = true;
          }
          await page.goto(`${origin}${route.path}`, { waitUntil: 'domcontentloaded', timeout: TIMEOUT });
        }
        await page.waitForLoadState('networkidle', { timeout: TIMEOUT }).catch(() => {});
        await page.waitForTimeout(300);

        const title = await page.title();
        const snapshot = await page._snapshotForAI({ timeout: TIMEOUT });
        pageReport.title = title;
        pageReport.snap = snapshot.full.replace(/\s*\[ref=\w+\]/g, '');
        pageReport.errors = pageConsole;
      } catch (e) {
        pageReport.error = e.message;
      }

      page.off('console', handler);
      report.pages.push(pageReport);
    }

    delete report._authenticated;
    const reportPath = '/tmp/dc-ux-tour.json';
    fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
    console.log(JSON.stringify({ reportPath, pagesVisited: report.pages.length, errors: report.pages.filter(p => p.error).length }, null, 2));
  } finally {
    await page.close();
    await browser.close();
  }
}

function selectBrowserType(name) {
  switch (name) {
    case 'chromium':
      return chromium;
    case 'firefox':
      return firefox;
    default:
      console.error(`ERROR: unsupported BROWSER_ENGINE="${name}". Use "chromium" or "firefox".`);
      process.exit(1);
  }
}

async function launchBrowser(browserType) {
  if (BROWSER_ENGINE === 'chromium') {
    return browserType.launch({
      headless: true,
      args: ['--no-sandbox', '--disable-setuid-sandbox']
    });
  }

  return browserType.launch({ headless: true });
}

main().catch(e => {
  console.error('ERROR:', e.message);
  process.exit(1);
});
