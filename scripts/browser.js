#!/usr/bin/env node
/**
 * Browser helper — uses Playwright's local headless browser (Chromium by default).
 *
 * Every invocation opens a NEW page, performs the operation, and closes it.
 * Works from any context: main session, subagents, CI.
 *
 * Usage:
 *   node scripts/browser.js snap   <url>              # aria/accessibility tree
 *   node scripts/browser.js shot   <url> [out.png]    # screenshot → /tmp/browser-shot.png
 *   node scripts/browser.js eval   <url> <js-expr>    # evaluate JS, print JSON result
 *   node scripts/browser.js errs   <url>              # console errors/warnings only
 *   node scripts/browser.js html   <url>              # raw page HTML (truncated to 50k)
 *   node scripts/browser.js click  <url> <selector>   # click element, return snap
 *   node scripts/browser.js fill   <url> <sel> <val>  # fill input, return snap
 *   node scripts/browser.js wait   <url> <selector>   # wait for selector, return snap
 *   node scripts/browser.js health <url>              # check page loads without errors
 *   node scripts/browser.js tour   --seed <phrase>    # visit key routes, save JSON
 *
 * Options:
 *   --seed <phrase>    Inject seed phrase into localStorage before navigating
 *   --viewport mobile  Use 375×812 (iPhone X) viewport
 *   --timeout <ms>     Override navigation timeout (default: 20000)
 *   --wait-api         Wait for /api/v1/ response after navigation
 *
 * Environment:
 *   BROWSER_TIMEOUT   Navigation timeout ms (default: 20000)
 *   DC_WEB_URL        Frontend URL (default: http://localhost:5173)
 *   BROWSER_ENGINE    "chromium" or "firefox" (default: chromium)
 *
 * Note: uses page._snapshotForAI() (Playwright 1.44+ internal) for snap.
 * Returns a YAML-like accessibility tree — far more useful than raw DOM.
 */

'use strict';

const { chromium, firefox } = require('/home/ubuntu/.npm-global/lib/node_modules/playwright');
const fs = require('fs');
const bip39 = require('/code/decent-cloud/website/node_modules/bip39');

const DEFAULT_TIMEOUT = 20000;
const TIMEOUT = parseInt(process.env.BROWSER_TIMEOUT || process.env.TIMEOUT || DEFAULT_TIMEOUT, 10);
const WEB_URL = process.env.DC_WEB_URL || 'http://localhost:5173';
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

const COMMANDS = ['snap', 'shot', 'eval', 'errs', 'html', 'click', 'fill', 'wait', 'health', 'tour'];

const [,, cmd, ...remainingArgs] = process.argv;

if (!cmd || !COMMANDS.includes(cmd)) {
  console.error('Usage: browser.js <snap|shot|eval|errs|html|click|fill|wait|health|tour> <url> [args...]');
  console.error('       browser.js tour --seed <phrase> [--viewport mobile]');
  console.error('Options:');
  console.error('  --seed <phrase>    Inject seed phrase for authenticated testing');
  console.error('  --viewport mobile  Use 375×812 (iPhone X) viewport');
  console.error('  --timeout <ms>     Override navigation timeout');
  console.error('  --wait-api         Wait for /api/v1/ response after navigation');
  process.exit(1);
}

let url = null;
let seedPhrase = null;
let viewport = null;
let waitApi = false;
let customTimeout = TIMEOUT;
let positionalArgs = [];

function parseArgs(args) {
  const result = { positional: [], seed: null, viewport: null, waitApi: false, timeout: TIMEOUT };
  
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    
    if (arg === '--seed') {
      const seedWords = [];
      for (let j = i + 1; j < args.length && !args[j].startsWith('--'); j++) {
        seedWords.push(args[j]);
      }
      const rawSeed = seedWords.join(' ').trim();
      if (rawSeed && bip39.validateMnemonic(rawSeed)) {
        result.seed = rawSeed;
      } else if (rawSeed) {
        console.error(`Error: Invalid BIP39 seed phrase. Got ${rawSeed.split(/\s+/).length} words: "${rawSeed.slice(0, 50)}..."`);
        console.error('Seed phrase must be 12 or 24 valid BIP39 words.');
        process.exit(1);
      }
    } else if (arg === '--viewport') {
      i++;
      if (args[i] === 'mobile') {
        result.viewport = MOBILE_VIEWPORT;
      }
    } else if (arg === '--wait-api') {
      result.waitApi = true;
    } else if (arg === '--timeout') {
      i++;
      result.timeout = parseInt(args[i], 10) || TIMEOUT;
    } else if (!arg.startsWith('--')) {
      result.positional.push(arg);
    }
  }
  
  return result;
}

const parsed = parseArgs(remainingArgs);
url = parsed.positional[0];
positionalArgs = parsed.positional.slice(1);
seedPhrase = parsed.seed;
viewport = parsed.viewport;
waitApi = parsed.waitApi;
customTimeout = parsed.timeout;

if (cmd !== 'tour' && !url) {
  console.error('Error: URL is required for non-tour commands');
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

  page.setDefaultTimeout(customTimeout);

  const consoleLogs = [];
  page.on('console', msg => {
    const type = msg.type();
    if (['error', 'warning', 'warn'].includes(type)) {
      consoleLogs.push(`[${type.toUpperCase()}] ${msg.text()}`);
    }
  });
  page.on('pageerror', err => consoleLogs.push(`[PAGEERROR] ${err.message}`));
  page.on('requestfailed', req => {
    const failure = req.failure();
    if (failure) {
      consoleLogs.push(`[REQUESTFAILED] ${req.url()} - ${failure.errorText}`);
    }
  });

  try {
    const fullUrl = url.startsWith('http') ? url : `${WEB_URL}${url}`;
    
    if (seedPhrase) {
      await authenticatePage(page, fullUrl, seedPhrase, customTimeout);
    } else {
      await page.goto(fullUrl, { waitUntil: 'domcontentloaded', timeout: customTimeout });
    }
    
    if (waitApi) {
      await page.waitForResponse(
        r => r.url().includes('/api/v1/'),
        { timeout: customTimeout }
      ).catch(() => {});
    }
    
    await page.waitForLoadState('networkidle', { timeout: customTimeout }).catch(e => {
      process.stderr.write(`[WARN] networkidle timeout: ${e.message}\n`);
    });

    switch (cmd) {
      case 'snap': {
        const snapshot = await page._snapshotForAI({ timeout: customTimeout });
        const clean = snapshot.full.replace(/\s*\[ref=\w+\]/g, '');
        process.stdout.write(clean + '\n');
        if (consoleLogs.length) {
          process.stdout.write('\n--- Console ---\n' + consoleLogs.join('\n') + '\n');
        }
        break;
      }

      case 'shot': {
        const outPath = positionalArgs[0] || '/tmp/browser-shot.png';
        await page.screenshot({ path: outPath, fullPage: false });
        console.log(outPath);
        break;
      }

      case 'eval': {
        const expr = positionalArgs.join(' ');
        const result = await page.evaluate(expr);
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

      case 'click': {
        const selector = positionalArgs[0];
        if (!selector) {
          console.error('Error: click requires a selector argument');
          process.exit(1);
        }
        await page.click(selector);
        await page.waitForLoadState('networkidle', { timeout: customTimeout }).catch(() => {});
        const snapshot = await page._snapshotForAI({ timeout: customTimeout });
        const clean = snapshot.full.replace(/\s*\[ref=\w+\]/g, '');
        process.stdout.write(clean + '\n');
        if (consoleLogs.length) {
          process.stdout.write('\n--- Console ---\n' + consoleLogs.join('\n') + '\n');
        }
        break;
      }

      case 'fill': {
        const selector = positionalArgs[0];
        const value = positionalArgs[1];
        if (!selector || value === undefined) {
          console.error('Error: fill requires selector and value arguments');
          process.exit(1);
        }
        await page.fill(selector, value);
        const snapshot = await page._snapshotForAI({ timeout: customTimeout });
        const clean = snapshot.full.replace(/\s*\[ref=\w+\]/g, '');
        process.stdout.write(clean + '\n');
        break;
      }

      case 'wait': {
        const selector = positionalArgs[0];
        if (!selector) {
          console.error('Error: wait requires a selector argument');
          process.exit(1);
        }
        await page.waitForSelector(selector, { timeout: customTimeout });
        const snapshot = await page._snapshotForAI({ timeout: customTimeout });
        const clean = snapshot.full.replace(/\s*\[ref=\w+\]/g, '');
        process.stdout.write(clean + '\n');
        break;
      }

      case 'health': {
        const result = {
          url: fullUrl,
          status: 'ok',
          title: await page.title(),
          errors: consoleLogs.filter(l => l.includes('[ERROR]') || l.includes('[PAGEERROR]')),
          warnings: consoleLogs.filter(l => l.includes('[WARNING]') || l.includes('[WARN]')),
        };
        
        if (result.errors.length > 0) {
          result.status = 'errors';
        } else if (result.warnings.length > 0) {
          result.status = 'warnings';
        }
        
        console.log(JSON.stringify(result, null, 2));
        break;
      }

      default:
        console.error(`Unknown command: ${cmd}. Use: ${COMMANDS.join(', ')}`);
        process.exit(1);
    }
  } finally {
    await page.close();
    await browser.close();
  }
}

async function authenticatePage(page, targetUrl, seedPhrase, timeout) {
  const origin = new URL(targetUrl).origin;
  
  await page.goto(origin, { waitUntil: 'domcontentloaded', timeout });

  await page.evaluate((phrase) => {
    const stored = JSON.parse(localStorage.getItem('seed_phrases') || '[]');
    if (!stored.includes(phrase)) {
      stored.push(phrase);
      localStorage.setItem('seed_phrases', JSON.stringify(stored));
    }
  }, seedPhrase);

  const authDone = page.waitForResponse(
    r => r.url().includes('/api/v1/accounts'),
    { timeout }
  );
  
  await page.goto(`${origin}/dashboard`, { waitUntil: 'domcontentloaded', timeout });
  await authDone;
  await page.waitForTimeout(200);

  if (targetUrl !== `${origin}/dashboard`) {
    await page.goto(targetUrl, { waitUntil: 'domcontentloaded', timeout });
    await page.waitForLoadState('networkidle', { timeout }).catch(() => {});
    await page.waitForTimeout(300);
  }
}

async function runTour(browserType) {
  const browser = await launchBrowser(browserType);
  const page = viewport
    ? await browser.newPage({ viewport })
    : await browser.newPage();

  page.setDefaultTimeout(customTimeout);
  
  const origin = new URL(WEB_URL).origin;
  const report = { timestamp: new Date().toISOString(), seed: seedPhrase ? '***' : null, viewport, pages: [] };

  try {
    for (const route of TOUR_ROUTES) {
      const pageReport = { path: route.path, name: route.name, url: `${origin}${route.path}` };
      process.stderr.write(`[TOUR] Visiting ${route.name} (${route.path})...\n`);

      const pageConsole = [];
      const handler = msg => { if (['error', 'warning', 'warn'].includes(msg.type())) pageConsole.push(`[${msg.type().toUpperCase()}] ${msg.text()}`); };
      page.on('console', handler);

      try {
        if (route.public && !seedPhrase) {
          await page.goto(`${origin}${route.path}`, { waitUntil: 'domcontentloaded', timeout: customTimeout });
        } else if (route.public) {
          await page.goto(`${origin}${route.path}`, { waitUntil: 'domcontentloaded', timeout: customTimeout });
        } else {
          if (seedPhrase && !report._authenticated) {
            await authenticatePage(page, `${origin}${route.path}`, seedPhrase, customTimeout);
            report._authenticated = true;
          } else {
            await page.goto(`${origin}${route.path}`, { waitUntil: 'domcontentloaded', timeout: customTimeout });
          }
        }
        await page.waitForLoadState('networkidle', { timeout: customTimeout }).catch(() => {});
        await page.waitForTimeout(300);

        const title = await page.title();
        const snapshot = await page._snapshotForAI({ timeout: customTimeout });
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
