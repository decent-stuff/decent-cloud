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

async function getAgent(identity) {
  if (!agent) {
    try {
      if (identity) {
        agent = await HttpAgent.create({
          host: defaultConfig.networkUrl,
          identity,
        });
        console.log('Agent created with identity:', identity);
      } else {
        agent = await HttpAgent.create({
          host: defaultConfig.networkUrl,
        });
        console.log('Agent created without identity');
      }
      await agent.fetchRootKey();
    } catch (error) {
      console.error(`Failed to initialize ${identity || 'anonymous'} HttpAgent`);
      throw error;
    }
  }
  return agent;
}

async function getCachedData(key) {
  const db = await initDB();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([storeName], 'readonly');
    const store = transaction.objectStore(storeName);
    const request = store.get(key);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const result = request.result;
      const age = Date.now() - result.timestamp;
      if (result && age < 600000) {
        // 10 minutes
        console.log(`[Cache] Found cached data, age: ${Math.round(age / 1000)} seconds`);
        resolve(result.data);
      } else {
        resolve(null);
      }
    };
  });
}

async function setCachedData(key, data) {
  const db = await initDB();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([storeName], 'readwrite');
    const store = transaction.objectStore(storeName);
    const request = store.put({
      key,
      data,
      timestamp: Date.now(),
    });

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.data);
  });
}

export async function fetchDataWithCache(cursor, bytesBefore, bypassCache = false) {
  const cacheKey = `data_fetch_${cursor}`;
  console.log(`[Cache] Fetching data for cursor: ${cursor}${bypassCache ? ' (bypass cache)' : ''}`);

  if (!bypassCache) {
    const cachedData = await getCachedData(cacheKey);
    if (cachedData) {
      return cachedData;
    } else {
      console.log('[Cache] No valid cached data found or cache expired');
    }
  } else {
    console.log('[Cache] Bypassing cache as requested');
  }

  console.log('[Cache] Fetching fresh data from canister');
  try {
    const result = await queryCanister('data_fetch', [[cursor], [bytesBefore]], {});
    console.log('[Cache] Successfully fetched fresh data, updating cache');
    await setCachedData(cacheKey, result);
    return result;
  } catch (error) {
    console.error('Error in fetchDataWithCache:', error);
    throw error;
  }
}

export async function queryCanister(methodName, args, options = {}) {
  try {
    const currentAgent = await getAgent();
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
    let currentAgent = await getAgent(identity);
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
