import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { nodePolyfills } from 'vite-plugin-node-polyfills';

export default defineConfig({
	plugins: [
		sveltekit(),
		nodePolyfills({
			include: ['buffer', 'util', 'stream']
		})
	],
	// Ensure Node.js built-ins are not polyfilled in SSR
	ssr: {
		noExternal: []
	},
	resolve: {
		alias: {
			// Ensure buffer uses the polyfill package, not Node.js module
			buffer: 'buffer/',
			// Fix @noble/curves import paths for older package version
			'@noble/curves/ed25519': '@noble/curves/ed25519.js'
		}
	}
});
