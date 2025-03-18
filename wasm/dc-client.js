/**
 * Main client module for the Decent Cloud WASM library.
 * This module provides the API for interacting with the ledger.
 *
 * This implementation ensures a single WASM instance is shared across:
 * - All components within the same page
 * - All pages within the same browser session
 * - All tabs within the same browser (when supported)
 */
import { __wbg_set_wasm } from './dc-client_bg.js';
import * as wasmModule from './dc-client_bg.js';

import { decentCloudLedger } from './ledger';
export { decentCloudLedger };

// Constants for cross-tab communication
const STORAGE_KEY = 'decent_cloud_wasm_initialized';
const BROADCAST_CHANNEL_NAME = 'decent_cloud_wasm_channel';

// Feature detection
const hasSharedWorker = typeof SharedWorker !== 'undefined';
const hasBroadcastChannel = typeof BroadcastChannel !== 'undefined';
// Check if localStorage is available without triggering errors
const hasLocalStorage = (function () {
  if (typeof window === 'undefined') return false;
  if (typeof localStorage === 'undefined') return false;

  // Additional check to handle security exceptions in some browsers
  try {
    const testKey = '__dc_test_key__';
    localStorage.setItem(testKey, '1');
    localStorage.removeItem(testKey);
    return true;
  } catch {
    return false;
  }
})();

// Module state
let initialized = false;
let initializationPromise = null;
let sharedWorker = null;
let broadcastChannel = null;

// Backward compatibility flag - set to true to disable cross-tab sharing
let disableCrossTabs = false;

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
 * Initialize the WASM module with cross-tab sharing capabilities.
 * @returns {Promise<void>} A promise that resolves when initialization is complete.
 */
export function initializeWasm(options = {}) {
  // Allow disabling cross-tab sharing via options
  if (options.disableCrossTabs) {
    disableCrossTabs = true;
  }

  // Return existing initialization if already in progress or completed
  if (initialized) return Promise.resolve();
  if (initializationPromise) return initializationPromise;

  // Create a promise to track initialization
  initializationPromise = _doInitializeWasm();
  return initializationPromise;
}

/**
 * Internal function to handle WASM initialization with cross-tab sharing.
 * @private
 * @returns {Promise<void>}
 */
async function _doInitializeWasm() {
  try {
    // Check if already initialized in this tab
    if (initialized) return;

    // If cross-tab sharing is disabled, use direct initialization
    if (disableCrossTabs) {
      await loadWasmModule();
      initialized = true;
      return;
    }

    // Try to use SharedWorker for cross-tab sharing if supported
    if (hasSharedWorker) {
      try {
        await initializeWithSharedWorker();
        return;
      } catch (workerError) {
        console.warn(
          'SharedWorker initialization failed, falling back to BroadcastChannel:',
          workerError
        );
      }
    }

    // Try to use BroadcastChannel as fallback
    if (hasBroadcastChannel) {
      try {
        await initializeWithBroadcastChannel();
        return;
      } catch (broadcastError) {
        console.warn(
          'BroadcastChannel initialization failed, falling back to localStorage:',
          broadcastError
        );
      }
    }

    // Fallback to localStorage for basic cross-page (same tab) sharing
    if (hasLocalStorage) {
      try {
        await initializeWithLocalStorage();
        return;
      } catch (storageError) {
        console.warn(
          'localStorage initialization failed, falling back to direct initialization:',
          storageError
        );
      }
    }

    // Final fallback: direct initialization
    await loadWasmModule();
    initialized = true;
  } catch (error) {
    console.error('All WASM initialization methods failed:', error);
    initializationPromise = null;
    throw error;
  }
}

/**
 * Initialize WASM using SharedWorker for cross-tab sharing.
 * @private
 * @returns {Promise<void>}
 */
function initializeWithSharedWorker() {
  return new Promise((resolve, reject) => {
    try {
      // Check if we're in a browser environment
      if (typeof window === 'undefined') {
        throw new Error('SharedWorker not available in this environment');
      }

      // Create inline worker if needed
      if (!window.URL || !window.URL.createObjectURL) {
        throw new Error('SharedWorker creation not supported in this browser');
      }

      // Get the worker URL or create it dynamically
      const workerUrl = getOrCreateWorkerUrl();

      // Create the shared worker
      sharedWorker = new SharedWorker(workerUrl);

      // Set up message handling
      sharedWorker.port.onmessage = async event => {
        const { type, data } = event.data;

        if (type === 'wasm-initialized') {
          // Worker already has WASM initialized
          initialized = true;
          resolve();
        } else if (type === 'initialize-wasm') {
          // Worker needs us to initialize WASM
          try {
            await loadWasmModule();
            sharedWorker.port.postMessage({ type: 'wasm-ready' });
            initialized = true;
            resolve();
          } catch (error) {
            reject(error);
          }
        } else if (type === 'error') {
          reject(new Error(data.message));
        }
      };

      // Handle errors
      sharedWorker.port.onerror = error => {
        reject(error);
      };

      // Start communication
      sharedWorker.port.start();
      sharedWorker.port.postMessage({ type: 'check-initialization' });
    } catch (error) {
      reject(error);
    }
  });
}

/**
 * Gets the worker URL or creates it dynamically if needed.
 * @private
 * @returns {string} The URL to the worker script.
 */
function getOrCreateWorkerUrl() {
  // First, try to use the external worker file
  try {
    const externalWorkerUrl = new URL('./dc-client-worker.js', import.meta.url).href;

    // Test if the file exists by making a HEAD request
    const xhr = new XMLHttpRequest();
    xhr.open('HEAD', externalWorkerUrl, false);
    xhr.send();

    if (xhr.status >= 200 && xhr.status < 300) {
      return externalWorkerUrl;
    }
  } catch (error) {
    console.warn('External worker file not found, creating inline worker:', error);
  }

  // If external file doesn't exist, create an inline worker
  return createInlineWorkerUrl();
}

/**
 * Creates an inline worker script as a Blob URL.
 * @private
 * @returns {string} The Blob URL for the worker script.
 */
function createInlineWorkerUrl() {
  // Check if we already created a worker script
  const existingScript = document.querySelector('script[data-worker-type="decent-cloud-worker"]');
  if (existingScript) {
    return existingScript.getAttribute('data-worker-blob-url');
  }

  // Create a blob URL for the worker script
  const workerScript = `
    // Shared state across all connections
    let wasmInitialized = false;
    const connections = [];

    // Handle new connections
    self.onconnect = function(e) {
      const port = e.ports[0];
      connections.push(port);

      port.onmessage = function(event) {
        const { type } = event.data;

        if (type === 'check-initialization') {
          if (wasmInitialized) {
            port.postMessage({ type: 'wasm-initialized' });
          } else {
            // Ask this connection to initialize WASM
            wasmInitialized = true;
            port.postMessage({ type: 'initialize-wasm' });
          }
        } else if (type === 'wasm-ready') {
          wasmInitialized = true;
          // Notify all other connections that WASM is ready
          connections.forEach(conn => {
            if (conn !== port) {
              conn.postMessage({ type: 'wasm-initialized' });
            }
          });
        }
      };

      port.start();
    };
  `;

  try {
    const blob = new Blob([workerScript], { type: 'application/javascript' });
    const workerUrl = URL.createObjectURL(blob);

    // Store the URL for cleanup later
    const script = document.createElement('script');
    script.setAttribute('data-worker-type', 'decent-cloud-worker');
    script.setAttribute('data-worker-blob-url', workerUrl);
    script.style.display = 'none';
    document.head.appendChild(script);

    return workerUrl;
  } catch (error) {
    console.warn('Failed to create inline worker script:', error);
    throw error;
  }
}

/**
 * Initialize WASM using BroadcastChannel for cross-tab communication.
 * @private
 * @returns {Promise<void>}
 */
function initializeWithBroadcastChannel() {
  return new Promise((resolve, reject) => {
    try {
      // Check if we're in a browser environment
      if (typeof window === 'undefined') {
        throw new Error('BroadcastChannel not available in this environment');
      }

      // Create broadcast channel
      broadcastChannel = new BroadcastChannel(BROADCAST_CHANNEL_NAME);

      // Set up message handling
      broadcastChannel.onmessage = event => {
        const { type } = event.data;

        if (type === 'wasm-initialized') {
          // Another tab has WASM initialized
          initialized = true;
          resolve();
        } else if (type === 'wasm-init-request') {
          // Another tab is asking if WASM is initialized
          if (initialized) {
            broadcastChannel.postMessage({ type: 'wasm-initialized' });
          }
        }
      };

      // Check if already initialized in localStorage
      const isInitialized = hasLocalStorage && localStorage.getItem(STORAGE_KEY) === 'true';

      if (isInitialized) {
        // Ask other tabs if they have WASM initialized
        broadcastChannel.postMessage({ type: 'wasm-init-request' });

        // Wait a short time for responses
        setTimeout(() => {
          if (!initialized) {
            // No other tab responded, initialize WASM ourselves
            loadWasmModule()
              .then(instance => {
                broadcastChannel.postMessage({ type: 'wasm-initialized' });
                initialized = true;
                resolve();
                return instance;
              })
              .catch(error => {
                if (broadcastChannel) {
                  broadcastChannel.close();
                  broadcastChannel = null;
                }
                reject(error);
              });
          }
        }, 100);
      } else {
        // Not initialized anywhere, do it now
        loadWasmModule()
          .then(instance => {
            if (hasLocalStorage) {
              try {
                localStorage.setItem(STORAGE_KEY, 'true');
              } catch (error) {
                console.warn('Failed to write to localStorage:', error);
              }
            }
            broadcastChannel.postMessage({ type: 'wasm-initialized' });
            initialized = true;
            resolve();
            return instance;
          })
          .catch(error => {
            if (broadcastChannel) {
              broadcastChannel.close();
              broadcastChannel = null;
            }
            reject(error);
          });
      }
    } catch (error) {
      if (broadcastChannel) {
        broadcastChannel.close();
        broadcastChannel = null;
      }
      reject(error);
    }
  });
}

/**
 * Initialize WASM using localStorage for basic cross-page sharing.
 * @private
 * @returns {Promise<void>}
 */
function initializeWithLocalStorage() {
  return new Promise((resolve, reject) => {
    try {
      // Check if localStorage is available
      if (!hasLocalStorage) {
        throw new Error('localStorage not available in this environment');
      }

      // Check if already initialized in this browser session
      let isInitialized = false;
      try {
        isInitialized = localStorage.getItem(STORAGE_KEY) === 'true';
      } catch (error) {
        console.warn('Error reading from localStorage:', error);
      }

      // Load the WASM module
      loadWasmModule()
        .then(instance => {
          initialized = true;

          // Try to update localStorage if not already initialized
          if (!isInitialized) {
            try {
              localStorage.setItem(STORAGE_KEY, 'true');
            } catch (error) {
              console.warn('Failed to write to localStorage:', error);
            }
          }

          resolve();
          return instance;
        })
        .catch(reject);
    } catch (error) {
      reject(error);
    }
  });
}

/**
 * Load the actual WASM module.
 * @private
 * @returns {Promise<void>}
 */
async function loadWasmModule() {
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
    return wasmModuleInstance;
  } catch (error) {
    console.error('Failed to load WASM module:', error);
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

/**
 * Sign data using ed25519.
 * @param {Uint8Array} secretKeyRaw - The private key, in raw format.
 * @param {Uint8Array} data - The data to sign.
 * @returns {Promise<Uint8Array>} The signature.
 */
export async function ed25519Sign(secretKeyRaw, data) {
  if (!initialized) await initializeWasm();
  return wasmModule.ed25519_sign(secretKeyRaw, data);
}
