const fs = require("fs");
const path = require("path");

// Ensure dist directory exists
const distPath = path.join(__dirname, "..", "dist");
if (!fs.existsSync(distPath)) {
  fs.mkdirSync(distPath, { recursive: true });
}

// Copy TypeScript definitions
const tsDefSource = path.join(__dirname, "..", "src", "js", "index.ts");
const tsDefDest = path.join(distPath, "index.d.ts");
fs.copyFileSync(tsDefSource, tsDefDest);

// Create package.json for dist
const pkg = require("../package.json");
const distPkg = {
  name: pkg.name,
  version: pkg.version,
  description: pkg.description,
  main: "index.js",
  types: "index.d.ts",
  license: pkg.license,
  repository: pkg.repository,
  files: ["*.js", "*.ts", "*.wasm"],
};

fs.writeFileSync(
  path.join(distPath, "package.json"),
  JSON.stringify(distPkg, null, 2)
);

// Create a simple test file
const testContent = `import { WasmLedgerMap } from '../src/js';

describe('LedgerMap', () => {
    let ledgerMap;

    beforeEach(async () => {
        ledgerMap = new WasmLedgerMap();
        await ledgerMap.initialize();
    });

    test('basic operations', async () => {
        const label = 'test';
        const key = new Uint8Array([1, 2, 3]);
        const value = new Uint8Array([4, 5, 6]);

        ledgerMap.beginBlock();
        ledgerMap.upsert(label, key, value);
        ledgerMap.commitBlock();

        const retrieved = ledgerMap.get(label, key);
        expect(retrieved).toEqual(value);

        expect(ledgerMap.getBlocksCount()).toBe(1);

        const hash = ledgerMap.getLatestBlockHash();
        expect(hash).toBeInstanceOf(Uint8Array);
        expect(hash.length).toBeGreaterThan(0);
    });
});
`;

const testDir = path.join(__dirname, "..", "tests");
if (!fs.existsSync(testDir)) {
  fs.mkdirSync(testDir, { recursive: true });
}

fs.writeFileSync(path.join(testDir, "ledger-map.test.ts"), testContent);

console.log("Build preparation completed successfully");
