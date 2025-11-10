import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { nodePolyfills } from 'vite-plugin-node-polyfills';

export default defineConfig({
	plugins: [
		sveltekit(),
		nodePolyfills({
			include: ['buffer', 'util', 'stream'],
			// Prevent polyfills from being injected into SSR/server code
			overrides: {
				fs: false
			}
		})
	],
	// Ensure Node.js built-ins are not polyfilled in SSR
	ssr: {
		noExternal: []
	},
	resolve: {
		alias: {
			// Ensure buffer uses the polyfill package, not Node.js module
			buffer: 'buffer/'
		}
	}
});
