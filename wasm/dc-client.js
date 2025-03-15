/**
 * Main client module for the Decent Cloud WASM library.
 * This module provides the API for interacting with the ledger.
 */
import { __wbg_set_wasm } from './dc-client_bg.js';
import * as wasmModule from './dc-client_bg.js';

import { ledger as DecentCloudLedger } from './ledger';
export { DecentCloudLedger };

// Track initialization state and module path
let initialized = false;

/**
 * Client class for interacting with the Decent Cloud ledger.
 */
export class DecentCloudClient {
  /**
   * Initialize the WASM module.
   * @returns {Promise<void>}
   */
  async initialize() {
    await initializeWasm();
  }

  /**
   * Clear the ledger storage.
   * @returns {Promise<void>}
   */
  async clearStorage() {
    return await ledgerStorageClear();
  }
}

/**
 * Initialize the WASM module.
 * @returns {Promise<void>} A promise that resolves when initialization is complete.
 */
export async function initializeWasm() {
  if (initialized) return;
  try {
    // 16 MB initial, up to 256 MB
    const memory = new WebAssembly.Memory({ initial: 256, maximum: 4096 });

    const response = await fetch(new URL('./dc-client_bg.wasm', import.meta.url));
    const wasmModuleInstance = await WebAssembly.instantiate(await response.arrayBuffer(), {
      env: { memory },
      './dc-client_bg.js': wasmModule,
    });
    __wbg_set_wasm(wasmModuleInstance.instance.exports);
    wasmModuleInstance.instance.exports.init();
    initialized = true;
  } catch (error) {
    console.error('Failed to initialize WASM module:', error);
    throw error;
  }
}

/**
 * Parse ledger blocks from raw binary input data.
 * @param {Uint8Array} inputData - The raw input data.
 * @param {bigint} [startOffset=0n] - The starting offset.
 * @returns {Promise<BlockData[]>} A promise that resolves to an array of ledger block data.
 */
export async function parseLedgerBlocks(inputData, startOffset) {
  if (!initialized) await initializeWasm();
  try {
    // The WASM function returns a JSON string; parse it into an object.
    const result = wasmModule.parse_ledger_blocks(inputData, startOffset);
    return typeof result === 'string' ? JSON.parse(result) : result;
  } catch (error) {
    console.error('Error in parseLedgerBlocks:', error);
    throw error;
  }
}

/**
 * Clear the ledger storage.
 * @returns {Promise<void>}
 */
export async function ledgerStorageClear() {
  if (!initialized) await initializeWasm();
  try {
    wasmModule.ledger_storage_clear();
  } catch (error) {
    console.error('Error in ledgerStorageClear:', error);
    throw error;
  }
}
