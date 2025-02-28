import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// Ensure we're in the right directory
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const wasmDir = __dirname;
process.chdir(wasmDir);

// Helper function to ensure directory exists
function ensureDirectoryExists(dir) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

// Helper function to copy file with error handling
function copyFile(src, dest) {
  try {
    fs.copyFileSync(src, dest);
    console.log(`Copied ${path.basename(src)} to ${path.relative(wasmDir, dest)}`);
  } catch (error) {
    console.error(`Failed to copy ${src}: ${error.message}`);
    throw error;
  }
}

console.log('🚀 Building WASM module...');

async function main() {
  try {
    // Ensure dist directory exists and is empty
    const distDir = path.join(wasmDir, 'dist');
    if (fs.existsSync(distDir)) {
      fs.rmSync(distDir, { recursive: true });
    }
    ensureDirectoryExists(distDir);

    // Run wasm-pack build with specific configuration
    console.log('Running wasm-pack build...');
    execSync('wasm-pack build --target bundler --out-dir dist --out-name dc-client --release', {
      stdio: 'inherit',
      env: {
        ...process.env,
        RUSTFLAGS: '-C opt-level=s --cfg getrandom_backend="wasm_js"',
        WASM_PACK_ARGS: '--verbose',
      },
    });

    // Clean up unnecessary files
    console.log('Cleaning up dist directory...');
    const filesToRemove = ['.gitignore', 'README.md', 'package.json'];

    filesToRemove.forEach(file => {
      const filePath = path.join(distDir, file);
      if (fs.existsSync(filePath)) {
        fs.unlinkSync(filePath);
        console.log(`Removed ${file}`);
      }
    });

    // Copy necessary files
    console.log('Copying additional files...');
    const filesToCopy = [
      ['index.d.ts', 'dist/dc-client.d.ts'],
      ['agent_js_wrapper.js', 'dist/agent_js_wrapper.js'],
      ['canister_idl.js', 'dist/canister_idl.js'],
      ['client.js', 'dist/dc-client.js'],
      ['client.js', 'dist/dc-client.mjs'],
    ];

    filesToCopy.forEach(([src, dest]) => {
      copyFile(path.join(wasmDir, src), path.join(wasmDir, dest));
    });

    // Wait for wasm-pack to create the snippets directory
    const snippetsDir = path.join(distDir, 'snippets');
    let retries = 0;
    while (!fs.existsSync(snippetsDir) && retries < 10) {
      await new Promise(resolve => {
        setTimeout(resolve, 100);
      });
      retries++;
    }

    if (fs.existsSync(snippetsDir)) {
      // Find the generated snippets subdirectory
      const snippetSubdirs = fs.readdirSync(snippetsDir);
      const wasmSnippetDir = snippetSubdirs.find(dir => dir.startsWith('decent-cloud-wasm-'));

      if (wasmSnippetDir) {
        // Copy canister_idl.js to the snippets directory
        const snippetDestPath = path.join(snippetsDir, wasmSnippetDir, 'canister_idl.js');
        copyFile(path.join(wasmDir, 'canister_idl.js'), snippetDestPath);
      }
    }

    // We're now using client.js as the main entry point
    console.log('Using client.js as the main entry point');

    console.log('✨ Build completed successfully!');
  } catch (error) {
    console.error('❌ Build failed:', error);
    process.exit(1);
  }
}

main().catch(error => {
  console.error('❌ Build failed:', error);
  process.exit(1);
});
