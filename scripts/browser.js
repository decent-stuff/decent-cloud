#!/usr/bin/env node
/**
 * CDP browser helper — wraps the container's Playwright to interact with the
 * real Chrome browser at http://192.168.0.13:9223.
 *
 * Every invocation opens a NEW tab, performs the operation, and closes the tab.
 * No external tab management needed. Works from any context (main session, subagents).
 *
 * Usage:
 *   node scripts/browser.js snap  <url>                  # visible text + structure
 *   node scripts/browser.js shot  <url> [out.png]        # screenshot (default: /tmp/browser-shot.png)
 *   node scripts/browser.js eval  <url> <js-expression>  # evaluate JS, print JSON result
 *   node scripts/browser.js errs  <url>                  # console errors/warnings only
 *   node scripts/browser.js html  <url>                  # raw page HTML (truncated to 50k)
 *
 * Environment:
 *   BROWSER_CDP_URL  Override CDP endpoint (default: http://192.168.0.13:9223)
 *   BROWSER_TIMEOUT  Navigation timeout ms (default: 20000)
 */

'use strict';

const { chromium } = require('/home/ubuntu/.npm-global/lib/node_modules/playwright');

const CDP     = process.env.BROWSER_CDP_URL || 'http://192.168.0.13:9223';
const TIMEOUT = parseInt(process.env.BROWSER_TIMEOUT || '20000', 10);

const [,, cmd, url, ...rest] = process.argv;

if (!cmd || !url) {
  console.error('Usage: browser.js <snap|shot|eval|errs|html> <url> [args...]');
  process.exit(1);
}

async function main() {
  const browser = await chromium.connectOverCDP(CDP);
  const ctx = browser.contexts()[0];
  const page = await ctx.newPage();

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

    switch (cmd) {
      case 'snap': {
        // Visible text structured by role/tag — cheap and readable
        const text = await page.evaluate(() => {
          const SKIP = new Set(['script', 'style', 'noscript', 'svg', 'path', 'head']);
          const lines = [];
          const walk = (node, depth) => {
            if (node.nodeType === Node.TEXT_NODE) {
              const t = node.textContent.trim();
              if (t) lines.push('  '.repeat(depth) + t);
              return;
            }
            if (node.nodeType !== Node.ELEMENT_NODE) return;
            const tag = node.tagName.toLowerCase();
            if (SKIP.has(tag)) return;
            const attrs = [];
            const role = node.getAttribute('role');
            const label = node.getAttribute('aria-label') || node.getAttribute('placeholder') || node.getAttribute('title');
            const href = tag === 'a' ? node.getAttribute('href') : null;
            if (role)  attrs.push(`role="${role}"`);
            if (label) attrs.push(`label="${label}"`);
            if (href)  attrs.push(`href="${href}"`);
            const attrStr = attrs.length ? ' ' + attrs.join(' ') : '';
            const isBlock = ['div','section','main','header','footer','nav','article','aside','form','ul','ol','table','tr','td','th','li','h1','h2','h3','h4','h5','h6','p','button','a','input','select','textarea','label'].includes(tag);
            if (isBlock && node.children.length > 0) {
              lines.push('  '.repeat(depth) + `<${tag}${attrStr}>`);
              for (const child of node.childNodes) walk(child, depth + 1);
            } else {
              for (const child of node.childNodes) walk(child, depth);
            }
          };
          walk(document.body, 0);
          return lines.join('\n');
        });
        process.stdout.write(text.slice(0, 30000) + '\n');
        if (consoleLogs.length) {
          process.stdout.write('\n--- Console ---\n' + consoleLogs.join('\n') + '\n');
        }
        break;
      }

      case 'shot': {
        const outPath = rest[0] || '/tmp/browser-shot.png';
        await page.waitForLoadState('networkidle', { timeout: TIMEOUT }).catch(() => {});
        await page.screenshot({ path: outPath, fullPage: false });
        console.log(outPath);
        break;
      }

      case 'eval': {
        const js = rest.join(' ');
        /* eslint-disable no-eval */
        const result = await page.evaluate(js);
        console.log(JSON.stringify(result, null, 2));
        break;
      }

      case 'errs': {
        // Wait briefly for JS to run
        await page.waitForTimeout(2000);
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
