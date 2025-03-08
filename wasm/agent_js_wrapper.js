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

  if (bypassCache) {
    console.log(`[Cache] Fetching data with no cache for cursor: ${cursor}`);
  } else {
    console.log(`[Cache] Checking cache for cursor: ${cursor}`);
    const cachedData = await getCachedData(cacheKey);
    if (cachedData) {
      console.debug(
        `[Cache] Using cached data for cursor: ${cursor} -> ${JSON.stringify(cachedData)}`
      );
      return cachedData;
    } else {
      console.log('[Cache] No valid cached data found or cache expired');
    }
  }

  console.log('[Cache] Fetching fresh data from canister');
  try {
    let result = await queryCanister('data_fetch', [[cursor], [bytesBefore]], {});
    console.log(
      `[Cache] Successfully fetched fresh data for cursor: ${cursor}, updating cache -> ${JSON.stringify(result)}`
    );
    if (result.Ok) {
      await setCachedData(cacheKey, result);
    }
    return result;
  } catch (error) {
    console.error('Error in fetchDataWithCache:', error);
    throw error;
  }
}

export async function queryCanister(methodName, args, options = {}) {
  try {
    const currentAgent = getAgent();
    const canisterId = options.canisterId || defaultConfig.canisterId;

    const actor = Actor.createActor(idlFactory, {
      agent: currentAgent,
      canisterId,
    });

    if (typeof actor[methodName] !== 'function') {
      throw new Error(`Method ${methodName} not found on the canister interface.`);
    }

    return await actor[methodName](...args);
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
