import { defineConfig } from 'vitest/config';
import { sveltekit } from '@sveltejs/kit/vite';

export default defineConfig({
	plugins: [sveltekit()],
	// Prefer the client ("browser") build of Svelte so lifecycle APIs
	// (onMount/mount) are available to component render tests via
	// @testing-library/svelte. Without this, vite-plugin-svelte compiles
	// components for SSR in vitest and `mount(...)` throws
	// `lifecycle_function_unavailable`.
	resolve: {
		conditions: ['browser']
	},
	test: {
		environment: 'jsdom',
		globals: true,
		include: ['src/**/*.{test,spec}.{js,ts}'],
		setupFiles: ['./src/test/setup.ts']
	}
});
