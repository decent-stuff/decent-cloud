/** @type {import('next').NextConfig} */
import { execSync } from 'child_process';
import { existsSync } from 'fs';
import { join } from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

// Get current directory in ES module
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Check if wasm package needs to be built
const wasmDistPath = join(__dirname, '../wasm/dist');
const wasmPackageJsonPath = join(wasmDistPath, 'package.json');

// Run wasm build if dist directory doesn't exist or is missing package.json
if (!existsSync(wasmDistPath) || !existsSync(wasmPackageJsonPath)) {
  console.log('Building @decent-stuff/dc-client package...');
  try {
    execSync('cd ../wasm && npm run build', { stdio: 'inherit' });
  } catch (error) {
    console.error('Failed to build @decent-stuff/dc-client package:', error);
    process.exit(1);
  }
} else {
  console.log('@decent-stuff/dc-client package already built');
}

const nextConfig = {
  images: {
    unoptimized: true,
  },
  // Uncomment this line if you want to use static export in the future
  // output: "export",
};

export default nextConfig;
