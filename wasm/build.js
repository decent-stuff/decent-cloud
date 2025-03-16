import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// Ensure we're in the right directory
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const wasmDir = __dirname;
process.chdir(wasmDir);

/**
 * Ensures a directory exists.
 * @param {string} dir - Directory path.
 */
function ensureDirectoryExists(dir) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

/**
 * Copies a file with error handling.
 * @param {string} src - Source file path.
 * @param {string} dest - Destination file path.
 */
function copyFile(src, dest) {
  try {
    fs.copyFileSync(src, dest);
    console.log(`Copied ${path.basename(src)} to ${path.relative(wasmDir, dest)}`);
  } catch (error) {
    console.error(`Failed to copy ${src}: ${error.message}`);
    throw error;
  }
}

/**
 * Checks if the source file is newer than the target file.
 * @param {string} src - Source file path.
 * @param {string} target - Target file path.
 * @returns {boolean} True if target does not exist or source is newer.
 */
function isNewer(src, target) {
  if (!fs.existsSync(target)) return true;
  const srcTime = fs.statSync(src).mtimeMs;
  const targetTime = fs.statSync(target).mtimeMs;
  return srcTime > targetTime;
}

/**
 * Uses the shell's `find` command to list all Rust (.rs) files in a directory that are newer than a target file.
 * @param {string} target - The target file path to compare against.
 * @param {string} dir - The directory to search in.
 * @returns {string[]} Array of file paths.
 */
function getNewerRustFiles(target, dir) {
  try {
    const cmd = `find ${dir} -type f -name '*.rs' -newer ${target}`;
    const output = execSync(cmd, { encoding: 'utf8' });
    return output.split('\n').filter(line => line.trim() !== '');
  } catch (err) {
    console.warn('Error running find command:', err);
    return [];
  }
}

/**
 * Retrieves the latest commit time (in ms) for any Rust file in the repository.
 * First, checks for uncommitted changes via `git status --porcelain`. If any Rust file
 * is modified, returns the current time; otherwise, uses git log.
 *
 * @returns {number} Timestamp in milliseconds.
 */
function getNewestGitRustFile() {
  try {
    // Check for uncommitted changes for .rs files from the git repository root.
    const gitRoot = execSync('git rev-parse --show-toplevel', { encoding: 'utf8' }).trim();
    const status = execSync(`git -C ${gitRoot} status --porcelain`, { encoding: 'utf8' });

    let newest = 0;
    status
      .split('\n')
      .filter(line => line.trim() !== '')
      .filter(line => fs.existsSync(path.join(gitRoot, line.slice(3).trim())))
      .filter(line => line.slice(3).trim().endsWith('.rs'))
      .some(line => {
        // The file path starts at position 3 in the output.
        const file = path.join(gitRoot, line.slice(3).trim());
        const mtime = fs.statSync(file).mtimeMs;
        newest = Math.max(newest, mtime);
      });
    return newest;
  } catch (err) {
    console.warn('Could not get latest rust commit time:', err);
    return 0;
  }
}

console.log('🚀 Building WASM module...');

async function main() {
  try {
    const gitRoot = execSync('git rev-parse --show-toplevel', { encoding: 'utf8' }).trim();
    const distDir = path.join(wasmDir, 'dist');
    ensureDirectoryExists(distDir);

    // Determine if we need to run wasm-pack build.
    const wasmTarget = path.join(distDir, 'dc-client_bg.wasm');
    let needWasmBuild = true;
    if (fs.existsSync(wasmTarget)) {
      // Use the shell command to get Rust files newer than wasmTarget.
      const newerRustFiles = getNewerRustFiles(wasmTarget, gitRoot);
      const fsCheck = newerRustFiles.length > 0;
      console.log(`Found ${newerRustFiles.length} newly modified Rust files:`, newerRustFiles);

      // Use Git commit timestamps.
      const wasmTargetMtime = fs.statSync(wasmTarget).mtimeMs;
      const gitLatestTime = getNewestGitRustFile();
      if (gitLatestTime > wasmTargetMtime) {
        console.log(
          `Git commit time: ${new Date(gitLatestTime).toISOString()} is newer than wasmTargetMtime: ${new Date(wasmTargetMtime).toISOString()}`
        );
      } else {
        console.log(
          `Git commit time: ${new Date(gitLatestTime).toISOString()} is older than wasmTargetMtime: ${new Date(wasmTargetMtime).toISOString()}`
        );
      }

      needWasmBuild = fsCheck || gitLatestTime > wasmTargetMtime;
    }

    if (needWasmBuild) {
      console.log('Running wasm-pack build...');
      execSync('wasm-pack build --target bundler --out-dir dist --out-name dc-client --release', {
        stdio: 'inherit',
        env: {
          ...process.env,
          RUSTFLAGS: '-C opt-level=s --cfg getrandom_backend="wasm_js"',
          WASM_PACK_ARGS: '--verbose',
        },
      });
    } else {
      console.log('Skipping wasm-pack build; no changes in Rust sources detected.');
    }

    // Clean up unnecessary files from dist.
    console.log('Cleaning up dist directory...');
    const filesToRemove = ['.gitignore'];
    filesToRemove.forEach(file => {
      const filePath = path.join(distDir, file);
      if (fs.existsSync(filePath)) {
        fs.unlinkSync(filePath);
        console.log(`Removed ${file}`);
      }
    });

    // Read main package.json metadata.
    console.log('Reading package.json for metadata...');
    const mainPackageJsonPath = path.join(wasmDir, 'package.json');
    const mainPackageJson = JSON.parse(fs.readFileSync(mainPackageJsonPath, 'utf8'));

    // Create package.json in dist.
    console.log('Creating package.json in dist directory...');
    const packageJson = {
      name: mainPackageJson.name,
      version: mainPackageJson.version,
      description: mainPackageJson.description,
      main: 'dc-client.js',
      types: 'dc-client.d.ts',
      type: 'module',
      files: ['*.js', '*.mjs', '*.ts', '*.d.ts', '*.wasm', 'snippets', 'LICENSE'],
      keywords: mainPackageJson.keywords,
      author: mainPackageJson.author,
      license: mainPackageJson.license,
      repository: mainPackageJson.repository,
      bugs: mainPackageJson.bugs,
      homepage: mainPackageJson.homepage,
      dependencies: {
        '@dfinity/agent': '^2.3.0',
        '@dfinity/principal': '^2.3.0',
        dexie: '^4.0.11',
      },
    };
    fs.writeFileSync(
      path.join(distDir, 'package.json'),
      JSON.stringify(packageJson, null, 2),
      'utf8'
    );
    console.log('Created package.json in dist directory');

    const filesToCompile = [['db.ts'], ['agent.ts'], ['ledger.ts']];
    filesToCompile.forEach(([src]) => {
      // Check if the source file exists
      const srcPath = path.join(wasmDir, src);
      const dest = path.basename(src, '.ts') + '.js';
      const dstPath = path.join(distDir, dest);

      if (!fs.existsSync(dstPath) || (fs.existsSync(srcPath) && isNewer(srcPath, dstPath))) {
        console.log(`🚀 Compiling ${srcPath} to ${dstPath}...`);
        try {
          execSync(
            `tsc ${srcPath} --module es2020 --target es2020 --moduleResolution node --outDir ${distDir}`,
            {
              stdio: 'inherit',
            }
          );

          console.log(`✅ Compiled ${srcPath} to ${dstPath}`);
          copyFile(srcPath, path.join(distDir, src));
        } catch (error) {
          console.error(`❌ Error compiling ${srcPath}: ${error.message}`);
          process.exit(1);
        }
      }
    });

    // Copy additional files only if their source is newer than the target.
    console.log('Copying additional files...');
    const filesToCopy = [
      ['canister_idl.js', 'canister_idl.js'],
      ['dc-client.js', 'dc-client.js'],
      ['dc-client.d.ts', 'dc-client.d.ts'],
      ['db.ts', 'db.ts'],
      ['agent.ts', 'agent.ts'],
      ['ledger.ts', 'ledger.ts'],
      ['LICENSE', 'LICENSE'],
    ];

    filesToCopy.forEach(([src, dest]) => {
      const srcPath = path.join(wasmDir, src);
      const dstPath = path.join(distDir, dest);
      copyFile(srcPath, dstPath);
    });

    // Wait for wasm-pack to create the snippets directory.
    const snippetsDir = path.join(distDir, 'snippets');
    let retries = 0;
    while (!fs.existsSync(snippetsDir) && retries < 10) {
      await new Promise(resolve => {
        setTimeout(resolve, 100);
      });
      retries++;
    }
    if (fs.existsSync(snippetsDir)) {
      const snippetSubdirs = fs.readdirSync(snippetsDir);
      const wasmSnippetDir = snippetSubdirs.find(dir => dir.startsWith('decent-cloud-wasm-'));
      if (wasmSnippetDir) {
        const snippetDestPath = path.join(snippetsDir, wasmSnippetDir, 'canister_idl.js');
        copyFile(path.join(wasmDir, 'canister_idl.js'), snippetDestPath);
      }
    }

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
