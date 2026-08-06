// Target configuration: resolution + LOUD validation.
//
// LOUD-FAIL CONTRACT (non-negotiable): if a selected flow needs a config field
// that is missing/empty/placeholder, the harness prints a precise error naming
// the field + its env var, then exits(2). It NEVER silently reports success.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HARNESS_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

/** Known target names (each should have a targets/<name>.json). */
export const KNOWN_TARGETS = ['dev', 'stage', 'prod'];

/** Sentinel used in example target files for secret fields. Treated as missing. */
export const PLACEHOLDER = 'PLACEHOLDER';

/**
 * @typedef {Object} TargetConfig
 * @property {string} target              env name (dev|stage|prod)
 * @property {string} webUrl              website origin (sign-up flow)
 * @property {string} apiUrl              API origin (https://api.decent-cloud.org)
 * @property {string} [hetznerToken]      real Hetzner API token (provider onboarding)
 * @property {string} [accountEmailPrefix] prefix for unique sign-up emails
 * @property {boolean} [includeProvision] enable the rent→provision→cancel flow (COSTS MONEY)
 * @property {string} [expectedEnvironment] expected /health `environment` value
 * @property {string} [source]            human label of where config came from
 */

/** Map a config key to its environment-variable name. */
export const ENV_FOR_KEY = {
	target: 'DC_E2E_TARGET',
	webUrl: 'DC_E2E_WEB_URL',
	apiUrl: 'DC_E2E_API_URL',
	hetznerToken: 'DC_E2E_HETZNER_TOKEN',
	accountEmailPrefix: 'DC_E2E_ACCOUNT_EMAIL_PREFIX',
	includeProvision: 'DC_E2E_INCLUDE_PROVISION',
	expectedEnvironment: 'DC_E2E_EXPECTED_ENVIRONMENT',
};

/**
 * Load config. Precedence (highest wins): env var > explicit --config file >
 * targets/<target>.json. Env vars override file values so CI/secret managers
 * can inject keys without editing files.
 *
 * @param {{target?: string, configPath?: string, env?: Record<string,string>}} opts
 * @returns {Promise<TargetConfig & {source:string}>}
 */
export async function loadConfig({ target, configPath, env = process.env }) {
	// 1. Target name: CLI flag wins, else env.
	const targetName = target ?? env.DC_E2E_TARGET ?? '';
	/** @type {TargetConfig & {source:string}} */
	let cfg = { target: String(targetName).trim(), source: '' };

	// 2. File layer (explicit path, else targets/<name>.json).
	let fileCfg = null;
	if (configPath) {
		if (!fs.existsSync(configPath)) {
			throw fatal(`Config file not found: ${configPath}`);
		}
		fileCfg = JSON.parse(fs.readFileSync(configPath, 'utf8'));
		cfg.source = configPath;
	} else if (targetName) {
		const targetFile = path.join(HARNESS_ROOT, 'targets', `${targetName}.json`);
		if (fs.existsSync(targetFile)) {
			fileCfg = JSON.parse(fs.readFileSync(targetFile, 'utf8'));
			cfg.source = `${path.relative(HARNESS_ROOT, targetFile)} (+ env overrides)`;
		}
	}
	if (fileCfg) {
		cfg = { ...cfg, ...fileCfg, target: cfg.target || fileCfg.target };
	}

	// 3. Env overrides (non-empty env values replace file values).
	for (const [key, envName] of Object.entries(ENV_FOR_KEY)) {
		const v = env[envName];
		if (v !== undefined && v !== '') {
			if (key === 'includeProvision') {
				cfg.includeProvision = ['1', 'true', 'yes', 'on'].includes(v.toLowerCase());
			} else {
				cfg[key] = v;
			}
		}
	}
	if (!cfg.source) cfg.source = 'env-only';
	return cfg;
}

/** Is a value "absent" (missing, empty, or the PLACEHOLDER sentinel)? */
function isAbsent(v) {
	return v === undefined || v === null || (typeof v === 'string' && v.trim() === '') || v === PLACEHOLDER;
}

/**
 * Validate that every key in `requiredKeys` is present + non-placeholder.
 * Returns a list of human-readable error strings (empty = valid).
 *
 * @param {TargetConfig} cfg
 * @param {string[]} requiredKeys
 * @returns {string[]}
 */
export function validateConfig(cfg, requiredKeys) {
	const errors = [];
	for (const key of requiredKeys) {
		if (isAbsent(cfg[key])) {
			const envName = ENV_FOR_KEY[key] ?? key;
			const reason = isAbsent(cfg[key]) && cfg[key] === PLACEHOLDER ? 'is the PLACEHOLDER sentinel' : 'is missing or empty';
			errors.push(
				`FATAL: missing required config '${key}' (env: ${envName}) — ${reason}. ` +
					hintForKey(key),
			);
		}
	}
	return errors;
}

/** Actionable hint per missing field (helps the operator fix it fast). */
function hintForKey(key) {
	switch (key) {
		case 'target':
			return `Set DC_E2E_TARGET to one of: ${KNOWN_TARGETS.join(', ')}.`;
		case 'webUrl':
			return 'Cannot drive the sign-up flow without the website URL.';
		case 'apiUrl':
			return 'Cannot reach the decent-cloud API.';
		case 'hetznerToken':
			return 'Cannot test provider onboarding (POST /cloud-accounts validates the token live).';
		case 'accountEmailPrefix':
			return 'Cannot generate unique sign-up emails.';
		default:
			return 'Required for the selected flow(s).';
	}
}

/** Config keys each flow needs (single source of truth for required-field sets). */
export const FLOW_REQUIREMENTS = {
	health: ['apiUrl', 'target'],
	marketplace: ['apiUrl'],
	'console-errors': ['webUrl'],
	drift: ['apiUrl', 'target'],
	'stats-honesty': ['apiUrl'],
	signup: ['webUrl', 'apiUrl', 'accountEmailPrefix'],
	'provider-onboard-path-a': ['webUrl', 'apiUrl', 'hetznerToken', 'accountEmailPrefix'],
	'rent-provision-cancel': ['webUrl', 'apiUrl', 'hetznerToken', 'accountEmailPrefix'],
};

/** Print message(s) to stderr and exit non-zero. */
export function fatal(msg, code = 2) {
	const lines = Array.isArray(msg) ? msg : [msg];
	for (const l of lines) process.stderr.write(l.endsWith('\n') ? l : l + '\n');
	process.exit(code);
	// unreachable, but keeps callers typed
	return new Error(msg);
}
