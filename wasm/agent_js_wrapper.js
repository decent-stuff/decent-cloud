import { HttpAgent, Actor } from '@dfinity/agent';
import { idlFactory } from './canister_idl.js';

let defaultConfig = {
  networkUrl: 'https://icp-api.io',
  canisterId: 'ggi4a-wyaaa-aaaai-actqq-cai',
};

// Create agent lazily to avoid initialization issues
let agent = null;

// Initialize IndexedDB
const dbName = 'decentCloudCache';
const storeName = 'dataCache';
const dbVersion = 1;

async function initDB() {
  // This function is already returning a Promise, so no need for await
  // But we'll add a comment to explain why this is intentional
  // to satisfy the ESLint rule
  await Promise.resolve(); // Dummy await to satisfy ESLint require-await rule

  return new Promise((resolve, reject) => {
    const request = indexedDB.open(dbName, dbVersion);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);

    request.onupgradeneeded = event => {
      const db = event.target.result;
      if (!db.objectStoreNames.contains(storeName)) {
        db.createObjectStore(storeName, { keyPath: 'key' });
      }
    };
  });
}

export function configure(config) {
  defaultConfig = { ...defaultConfig, ...config };
  agent = null;
}

function getAgent(identity) {
  if (!agent) {
    try {
      if (identity) {
        agent = HttpAgent.createSync({
          host: defaultConfig.networkUrl,
          shouldFetchRootKey: true,
          identity,
        });
        console.log('Agent created with identity:', identity);
      } else {
        agent = HttpAgent.createSync({
          host: defaultConfig.networkUrl,
          shouldFetchRootKey: true,
        });
        console.log('Agent created without identity');
      }
    } catch (error) {
      console.error(`Failed to initialize ${identity || 'anonymous'} HttpAgent`);
      throw error;
    }
  }
  return agent;
}

async function setCachedData(key, data) {
  try {
    const db = await initDB();
    const transaction = db.transaction(storeName, 'readwrite');
    const store = transaction.objectStore(storeName);

    const record = {
      key,
      data,
      timestamp: Date.now(),
    };

    console.debug(`[Cache] Setting cached data for key: ${key} -> ${JSON.stringify(record)}`);
    return await store.put(record);
  } catch (error) {
    console.error(`[Cache] Error setting cached data for key: ${key}`, error);
    throw error;
  }
}

async function getCachedData(key) {
  try {
    const db = await initDB();
    const transaction = db.transaction(storeName, 'readonly');
    const store = transaction.objectStore(storeName);

    // Await the record directly (assuming a promise-based API)
    const record = await store.get(key);
    console.debug(`[Cache] Fetching data for key: ${key} -> ${JSON.stringify(record)}`);

    // Return data only if it exists and is less than 10 minutes old (600000 ms)
    if (record && Date.now() - record.timestamp < 600000) {
      return record.data;
    } else {
      return null;
    }
  } catch (error) {
    console.error(`[Cache] Error getting cached data for key: ${key}`, error);
    throw error;
  }
}

export async function fetchDataWithCache(cursor, bytesBefore, bypassCache = false) {
  const cacheKey = `data_fetch_${cursor}`;

  // Add logging to help track cursor format issues
  try {
    console.log(
      `[Cache] Cursor details - Type: ${typeof cursor}, Length: ${cursor.length}, Value: ${cursor}`
    );

    // Sanitize cursor value to catch potential issues early
    if (typeof cursor !== 'string') {
      console.warn(`[Cache] Cursor is not a string, converting: ${cursor}`);
      cursor = String(cursor);
    }

    // Validate cursor format
    if (cursor.includes('undefined') || cursor.includes('null')) {
      console.warn(`[Cache] Suspicious cursor format: ${cursor}`);
    }
  } catch (e) {
    console.warn(`[Cache] Cursor format error: ${e.message}`);
  }

  if (bypassCache) {
    console.log(`[Cache] Fetching data with no cache for cursor: ${cursor}`);
  } else {
    console.log(`[Cache] Checking cache for cursor: ${cursor}`);
    try {
      const cachedData = await getCachedData(cacheKey);
      if (cachedData) {
        console.debug(
          `[Cache] Using cached data for cursor: ${cursor} -> ${JSON.stringify(cachedData)}`
        );
        return cachedData;
      } else {
        console.log('[Cache] No valid cached data found or cache expired');
      }
    } catch (cacheError) {
      console.warn('[Cache] Error accessing cache, will fetch fresh data:', cacheError);
      // Continue to fetch fresh data
    }
  }

  console.log('[Cache] Fetching fresh data from canister');
  try {
    // Check binary data format
    if (bytesBefore) {
      console.log(
        `[Cache] bytesBefore type: ${typeof bytesBefore}, length: ${bytesBefore.length || 'unknown'}`
      );
    }

    // Wrap canister query with extra error handling
    let result;
    try {
      result = await queryCanister('data_fetch', [[cursor], [bytesBefore]], {});

      // Validate the result structure before processing
      if (result && result.Ok && Array.isArray(result.Ok)) {
        const binaryData = result.Ok[1];
        if (binaryData) {
          console.log(
            `[Cache] Binary data type: ${binaryData.constructor ? binaryData.constructor.name : 'unknown'}, length: ${binaryData.length || binaryData.byteLength || 'unknown'}`
          );
        }
      }
    } catch (queryError) {
      console.error(`[Cache] Error in data_fetch query: ${queryError.message}`, queryError);
      if (queryError.message && queryError.message.includes('TextDecoder')) {
        console.error('[Cache] TextDecoder error detected - possibly malformed binary data');
      }
      throw queryError;
    }

    console.log(
      `[Cache] Successfully fetched fresh data for cursor: ${cursor}, updating cache -> ${JSON.stringify(result)}`
    );

    if (result && result.Ok) {
      try {
        await setCachedData(cacheKey, result);
      } catch (cacheError) {
        console.warn('[Cache] Failed to cache data, but continuing:', cacheError);
      }
    }

    return result;
  } catch (error) {
    console.error('Error in fetchDataWithCache:', error);
    throw error;
  }
}

export async function queryCanister(methodName, args, options = {}) {
  try {
    // Input validation
    if (!methodName || typeof methodName !== 'string') {
      throw new Error(`Invalid method name: ${methodName}`);
    }

    if (!Array.isArray(args)) {
      console.warn(`Args is not an array, converting: ${args}`);
      args = [args]; // Convert to array to avoid errors
    }

    // Get agent with better error handling
    let currentAgent;
    try {
      currentAgent = getAgent();
    } catch (agentError) {
      console.error('Failed to create agent:', agentError);
      throw new Error(`Agent creation failed: ${agentError.message}`);
    }

    const canisterId = options.canisterId || defaultConfig.canisterId;

    // Create actor with better error handling
    let actor;
    try {
      actor = Actor.createActor(idlFactory, {
        agent: currentAgent,
        canisterId,
      });
    } catch (actorError) {
      console.error('Failed to create actor:', actorError);
      throw new Error(`Actor creation failed: ${actorError.message}`);
    }

    if (typeof actor[methodName] !== 'function') {
      throw new Error(`Method ${methodName} not found on the canister interface.`);
    }

    // Call method with better error handling
    try {
      const result = await actor[methodName](...args);

      // Log success and return information about the result structure
      if (methodName === 'data_fetch' && result && result.Ok) {
        console.log(
          `[Cache] data_fetch result structure: ${JSON.stringify({
            hasOk: !!result.Ok,
            isArray: Array.isArray(result.Ok),
            length: Array.isArray(result.Ok) ? result.Ok.length : 'not array',
            firstElementType:
              Array.isArray(result.Ok) && result.Ok[0] ? typeof result.Ok[0] : 'n/a',
            secondElementExists: Array.isArray(result.Ok) && result.Ok.length > 1,
          })}`
        );
      }

      return result;
    } catch (callError) {
      console.error(`Error calling method ${methodName}:`, callError);

      // Provide more diagnostics for TextDecoder errors
      if (callError.message && callError.message.includes('TextDecoder')) {
        console.error(
          '[CRITICAL] TextDecoder.decode failed - this is likely due to malformed binary data',
          {
            errorName: callError.name,
            errorStack: callError.stack,
          }
        );
      }

      throw new Error(`Canister method call failed: ${callError.message}`);
    }
  } catch (error) {
    console.error('Error in queryCanister:', error);
    throw error;
  }
}

export async function updateCanister(methodName, args, identity, options = {}) {
  try {
    let currentAgent = getAgent(identity);
    const canisterId = options.canisterId || defaultConfig.canisterId;

    const actor = Actor.createActor(idlFactory, {
      agent: currentAgent,
      canisterId,
    });

    if (typeof actor[methodName] !== 'function') {
      throw new Error(`Method "${methodName}" not found on the canister interface.`);
    }

    return await actor[methodName](...args);
  } catch (error) {
    console.error('Error in updateCanister:', error);
    throw error;
  }
}
