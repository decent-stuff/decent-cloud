import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';

export default defineConfig({
  plugins: [wasm()],
  server: {
    fs: {
      // Allow serving files from one level up to the project root
      allow: ['..'],
    },
    port: 3000,
    open: true,
  },
  optimizeDeps: {
    esbuildOptions: {
      target: 'esnext',
    },
    include: ['@dfinity/agent', '@dfinity/principal'],
  },
  build: {
    target: 'esnext',
    assetsInlineLimit: 1024 * 1024, // 1MB inline limit, adjust as needed
    commonjsOptions: {
      transformMixedEsModules: true,
    },
  },
  // Configure Vite to properly handle WASM files
  assetsInclude: ['**/*.wasm'],
  define: {
    // Add polyfill for global
    // Fixes the error "global is not defined" from the @dfinity/agent package
    // which expects the "global" object to be available, which is typically present in Node.js environments but not in browsers
    global: 'globalThis',
  },
});
