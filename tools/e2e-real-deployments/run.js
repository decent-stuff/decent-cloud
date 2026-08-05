#!/usr/bin/env node
// dc-e2e: parametrized real-deployment e2e harness for decent-cloud.
//
// Usage:
//   node run.js --list
//   node run.js --target <dev|stage|prod> [--flows a,b,...] [--config path] [--include-provision]
//   DC_E2E_API_URL=... DC_E2E_TARGET=prod node run.js --flows health
//
// LOUD-FAIL CONTRACT: missing required inputs → precise stderr error naming the
// field + env var, then exit(2). Never silent success.

import { loadConfig, validateConfig, fatal, FLOW_REQUIREMENTS } from './src/config.js';
import { expandSelection, runFlows, summarize, listFlowNames } from './src/runner.js';

const BANNER = 'decent-cloud real-deployment e2e harness';

function parseArgs(argv) {
	const out = { list: false, target: undefined, configPath: undefined, flows: null, includeProvision: false, help: false };
	for (let i = 2; i < argv.length; i++) {
		const a = argv[i];
		switch (a) {
			case '--list':
				out.list = true;
				break;
			case '--target':
				out.target = argv[++i];
				break;
			case '--config':
				out.configPath = argv[++i];
				break;
			case '--flows':
				out.flows = (argv[++i] ?? '').split(',').map((s) => s.trim()).filter(Boolean);
				break;
			case '--include-provision':
				out.includeProvision = true;
				break;
			case '-h':
			case '--help':
				out.help = true;
				break;
			default:
				if (a.startsWith('--target=')) out.target = a.slice('--target='.length);
				else if (a.startsWith('--flows=')) out.flows = a.slice('--flows='.length).split(',').map((s) => s.trim()).filter(Boolean);
				else if (a.startsWith('--config=')) out.configPath = a.slice('--config='.length);
				else {
					stderr(`error: unknown argument '${a}'`);
					process.exit(2);
				}
		}
	}
	return out;
}

function stderr(s) {
	process.stderr.write(s.endsWith('\n') ? s : s + '\n');
}
function stdout(s) {
	process.stdout.write(s.endsWith('\n') ? s : s + '\n');
}

function usage() {
	stdout(`${BANNER}

Usage:
  node run.js --list
  node run.js --target <dev|stage|prod> [options]

Options:
  --target <name>        Target env (dev|stage|prod). Loads targets/<name>.json.
  --config <path>        Explicit config file (overrides --target file).
  --flows <a,b,...>      Comma-separated subset (default: all non-gated flows).
  --include-provision    Enable the rent-provision-cancel flow (SPENDS MONEY).

Config (env overrides file values):
  DC_E2E_TARGET, DC_E2E_WEB_URL, DC_E2E_API_URL,
  DC_E2E_HETZNER_TOKEN, DC_E2E_ACCOUNT_EMAIL_PREFIX,
  DC_E2E_INCLUDE_PROVISION, DC_E2E_EXPECTED_ENVIRONMENT

Loud-fail: missing required inputs for a selected flow → stderr names the field
+ env var, exit code 2. Findings are printed but do not fail the run; any flow
FAIL makes the exit code 1.`);
}

async function main() {
	const args = parseArgs(process.argv);
	if (args.help) {
		usage();
		return;
	}

	if (args.list) {
		stdout(`${BANNER} — available flows:`);
		for (const name of listFlowNames()) {
			const reqs = FLOW_REQUIREMENTS[name] ?? [];
			const gated = name === 'rent-provision-cancel' ? ' [GATED: --include-provision]' : '';
			stdout(`  ${name.padEnd(26)} requires: [${reqs.join(', ')}]${gated}`);
		}
		stdout(`\nDefault run (no --flows): all flows except rent-provision-cancel.`);
		return;
	}

	let config;
	try {
		config = await loadConfig({ target: args.target, configPath: args.configPath });
	} catch (e) {
		fatal(e.message || String(e));
	}

	if (args.includeProvision) config.includeProvision = true;

	// Resolve the selected flows (with prerequisites).
	let selectedFlows;
	try {
		selectedFlows = expandSelection(args.flows);
	} catch (e) {
		fatal(e.message);
	}

	const selectedNames = selectedFlows.map((f) => f.name);

	// LOUD validation: collect the required config keys across the selection and
	// check every one. This is the first "test" and always runs.
	const requiredKeys = new Set();
	for (const name of selectedNames) {
		for (const k of FLOW_REQUIREMENTS[name] ?? []) requiredKeys.add(k);
	}
	const errors = validateConfig(config, [...requiredKeys]);
	if (errors.length) {
		stderr(`${BANNER}\n`);
		stderr('FATAL: configuration validation failed — refusing to run. Missing/invalid inputs:\n');
		for (const e of errors) stderr('  ' + e);
		stderr('\nResolve the above and re-run. The harness never reports success when required inputs are missing.');
		process.exit(2);
	}

	// Header.
	stdout(`${BANNER}`);
	stdout(`  target : ${config.target}`);
	stdout(`  apiUrl : ${config.apiUrl}`);
	stdout(`  webUrl : ${config.webUrl ?? '(not required for selected flows)'}`);
	stdout(`  flows  : ${selectedNames.join(', ')}`);
	stdout(`  source : ${config.source}`);
	if (config.includeProvision) stdout(`  ⚠ includeProvision=true — rent-provision-cancel will spend real money.`);
	stdout('');

	const log = (s) => stdout(s);
	let failed = 0;
	const onResult = (r) => {
		const tag = r.status === 'pass' ? '[PASS]' : '[FAIL]';
		stdout(`${tag} ${r.name.padEnd(24)} — ${r.detail}`);
		for (const f of r.findings) stdout(`        [FINDING] ${f}`);
		if (r.status === 'fail') failed++;
	};

	const { results } = await runFlows({ config, selectedFlows, log, onResult });
	const summary = summarize(results);

	stdout('');
	stdout('─'.repeat(64));
	stdout(`SUMMARY: ${summary.passed}/${summary.total} passed, ${summary.failed} failed, ${summary.findings} finding(s)`);
	if (summary.exitCode !== 0) {
		stdout(`RESULT : FAIL (one or more flows failed)`);
	} else {
		stdout(`RESULT : PASS`);
	}
	process.exit(summary.exitCode);
}

main().catch((e) => {
	stderr(`\nFATAL: unhandled error: ${e?.stack || e?.message || String(e)}`);
	process.exit(3);
});
