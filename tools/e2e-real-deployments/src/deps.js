// Dependency loader: resolves third-party modules from this project's own
// node_modules FIRST, then falls back to the sibling website's node_modules.
//
// Why: the harness is a standalone project (declare deps in package.json), but
// it must ALSO run out-of-the-box WITHOUT `npm install` by reusing the repo's
// already-installed Playwright / @noble / bip39 under website/node_modules. This
// keeps `node run.js --list` working on a fresh clone with zero setup.
//
// All real deps are ESM-loaded. CJS modules (playwright) arrive as { default },
// which `unwrap()` exposes as the module.exports object.

import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const HARNESS_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const REPO_ROOT = path.resolve(HARNESS_ROOT, '../..');
const SEARCH_PATHS = [
	path.join(HARNESS_ROOT, 'node_modules'),
	path.join(REPO_ROOT, 'website/node_modules'),
];

/** Resolve a bare module specifier to an absolute file path. */
export function resolveModule(spec) {
	let lastErr;
	for (const dir of SEARCH_PATHS) {
		try {
			return createRequire(import.meta.url).resolve(spec, { paths: [dir] });
		} catch (e) {
			lastErr = e;
		}
	}
	// Last resort: bare resolution from this file (may pick up a global install).
	try {
		return createRequire(import.meta.url).resolve(spec);
	} catch {
		throw new Error(
			`Cannot resolve dependency '${spec}'. Run \`npm install\` in tools/e2e-real-deployments, ` +
				`or ensure website/node_modules is present. (last error: ${lastErr?.message ?? 'n/a'})`,
		);
	}
}

/** Dynamic-import a bare specifier, unwrapping CJS `.default` to module.exports. */
export async function loadModule(spec) {
	const resolved = resolveModule(spec);
	const mod = await import(pathToFileURL(resolved).href);
	// CJS modules imported as ESM expose their exports under `default`.
	return mod?.default && typeof mod.default === 'object' && !Array.isArray(mod.default)
		? { ...mod.default, ...mod }
		: mod;
}
