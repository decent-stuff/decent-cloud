import { __wbg_set_wasm } from './dc-client_bg.js';
import * as wasm from './dc-client_bg.js';
import {
  configure as configureAgent,
  queryCanister,
  updateCanister,
  fetchDataWithCache,
} from './agent_js_wrapper.js';

let initialized = false;

/**
 * LedgerStorage class for managing ledger storage operations
 */
class LedgerStorage {
  /**
   * Clear the storage system
   */
  clear() {
    return wasm.ledger_storage_clear();
  }

  /**
   * Get the size of the storage in bytes
   */
  sizeBytes() {
    return wasm.ledger_storage_size_bytes();
  }

  /**
   * Read data from storage at a specific offset
   * @param {number} offset The offset to read from
   * @param {number} length The number of bytes to read
   * @returns {Uint8Array} The data read from storage
   * @throws {Error} If reading from storage fails
   */
  readOffset(offset, length) {
    const result = wasm.ledger_storage_read_offset(offset, length);

    // Handle the Result type
    if (result && typeof result === 'object') {
      if ('Ok' in result) {
        return result.Ok;
      } else if ('Err' in result) {
        throw new Error(`Failed to read storage: ${result.Err}`);
      }
    }

    // Fallback for backward compatibility
    return result;
  }

  /**
   * Write data to storage at a specific offset
   * @param {number} offset The offset to write to
   * @param {Uint8Array} data The data to write
   */
  writeOffset(offset, data) {
    return wasm.ledger_storage_write_offset(offset, data);
  }
}

/**
 * LedgerOperations class for managing ledger data operations
 */
class LedgerOperations {
  /**
   * Get the local cursor as a string
   * @returns {string} The local cursor as a string
   */
  getCursorLocalAsString() {
    return wasm.ledger_cursor_local_as_str();
  }

  /**
   * Get a ledger block header and data as JSON
   * @param {bigint} blockOffset The block offset
   * @returns {string} The ledger block header and data (entries) as a JSON string
   */
  getBlockAsJson(blockOffset) {
    const result = wasm.ledger_get_block_as_json(blockOffset);

    // Ensure consistent return format as a string
    if (typeof result === 'string') {
      return result;
    } else if (result && typeof result === 'object' && 'Ok' in result) {
      return result.Ok;
    } else {
      return JSON.stringify(result);
    }
  }

  /**
   * Get a value from the ledger
   * @param {string} label The label
   * @param {Uint8Array} key The key as Uint8Array
   * @returns {Uint8Array|null} The value as Uint8Array if found, null otherwise
   */
  getValue(label, key) {
    try {
      return wasm.ledger_get_value(label, key);
    } catch {
      return null;
    }
  }

  /**
   * Set a value in the ledger
   * @param {string} label The label
   * @param {Uint8Array} key The key as Uint8Array
   * @param {Uint8Array} value The value as Uint8Array
   */
  setValue(label, key, value) {
    return wasm.ledger_set_value(label, key, value);
  }

  /**
   * Remove a value from the ledger
   * @param {string} label The label
   * @param {Uint8Array} key The key as Uint8Array
   */
  removeValue(label, key) {
    return wasm.ledger_remove_value(label, key);
  }

  /**
   * Get transactions
   * @returns {Promise<any>} A promise that resolves with the transactions
   */
  async getTransactions() {
    return await wasm.get_transactions();
  }
}

/**
 * CanisterInteraction class for interacting with the canister
 */
class CanisterInteraction {
  /**
   * Configure the canister interaction
   * @param {Object} config The configuration options
   */
  configure(config) {
    configureAgent(config);
  }

  /**
   * Generic query function that can be used for any query method
   * @param {string} methodName The name of the method to call
   * @param {any} args The arguments to pass to the method
   * @returns {Promise<any>} A promise that resolves with the result
   */
  async callQuery(methodName, args) {
    return await queryCanister(methodName, args);
  }

  /**
   * Generic update function that can be used for any update method
   * @param {string} methodName The name of the method to call
   * @param {any} args The arguments to pass to the method
   * @param {any} identity The identity to use for the call
   * @returns {Promise<any>} A promise that resolves with the result
   */
  async callUpdate(methodName, args, identity) {
    return await updateCanister(methodName, args, identity);
  }
}

/**
 * Main client class for interacting with Decent Cloud
 */
class DecentCloudClient {
  /**
   * Create a new DecentCloudClient instance
   * @param {Object} config Optional configuration options
   */
  constructor(config = {}) {
    this._storage = new LedgerStorage();
    this._ledger = new LedgerOperations();
    this._canister = new CanisterInteraction();

    if (config) {
      this._canister.configure(config);
    }
  }

  /**
   * Initialize the WASM module, storage system, and LedgerMap
   * @returns {Promise<string>} A promise that resolves when initialization is complete
   */
  async initialize() {
    if (!initialized) {
      const response = await fetch(new URL('./dc-client_bg.wasm', import.meta.url));
      const wasmModule = await WebAssembly.instantiate(await response.arrayBuffer(), {
        './dc-client_bg.js': wasm,
      });
      __wbg_set_wasm(wasmModule.instance.exports);
      const result = await wasmModule.instance.exports.initialize();
      initialized = true;
      return result;
    }
    return 'WASM module already initialized';
  }

  /**
   * Access to ledger storage operations
   * @returns {LedgerStorage} The ledger storage operations
   */
  get storage() {
    return this._storage;
  }

  /**
   * Access to ledger data operations
   * @returns {LedgerOperations} The ledger data operations
   */
  get ledger() {
    return this._ledger;
  }

  /**
   * Access to canister interaction methods
   * @returns {CanisterInteraction} The canister interaction methods
   */
  get canister() {
    return this._canister;
  }
}

/**
 * Create a new DecentCloudClient instance
 * @param {Object} config Optional configuration options
 * @returns {DecentCloudClient} A new DecentCloudClient instance
 */
export function createClient(config = {}) {
  return new DecentCloudClient(config);
}

// For backward compatibility, also export the original functions
export {
  wasm,
  initialized,
  DecentCloudClient,
  LedgerStorage,
  LedgerOperations,
  CanisterInteraction,
};

// Export the initialize function for backward compatibility
export async function initialize() {
  const client = new DecentCloudClient();
  return await client.initialize();
}

// Export the original functions from agent_js_wrapper.js for backward compatibility
export { configureAgent as configure, queryCanister, updateCanister, fetchDataWithCache };

// Export the ledger functions for backward compatibility
export async function ledger_storage_clear() {
  await initialize();
  return wasm.ledger_storage_clear();
}

// Export the storage read function with proper error handling
export async function ledger_storage_read_offset(offset, length) {
  await initialize();
  const result = wasm.ledger_storage_read_offset(offset, length);

  // Handle the Result type
  if (result && typeof result === 'object') {
    if ('Ok' in result) {
      return result.Ok;
    } else if ('Err' in result) {
      throw new Error(`Failed to read storage: ${result.Err}`);
    }
  }

  // Fallback for backward compatibility
  return result;
}

export async function ledger_get_value(label, key) {
  await initialize();
  try {
    return wasm.ledger_get_value(label, key);
  } catch {
    return null;
  }
}

export async function ledger_set_value(label, key, value) {
  await initialize();
  return wasm.ledger_set_value(label, key, value);
}

export async function ledger_remove_value(label, key) {
  await initialize();
  return wasm.ledger_remove_value(label, key);
}

export async function ledger_get_block_as_json(blockOffset) {
  await initialize();
  const result = wasm.ledger_get_block_as_json(blockOffset);

  // Ensure consistent return format as a string
  if (typeof result === 'string') {
    return result;
  } else if (result && typeof result === 'object' && 'Ok' in result) {
    return result.Ok;
  } else {
    return JSON.stringify(result);
  }
}

export async function get_transactions() {
  await initialize();
  return wasm.get_transactions();
}

export async function call_query_canister(methodName, args) {
  await initialize();
  const client = new DecentCloudClient();
  return client.canister.callQuery(methodName, args);
}

export async function call_update_canister(methodName, args, identity) {
  await initialize();
  const client = new DecentCloudClient();
  return client.canister.callUpdate(methodName, args, identity);
}
