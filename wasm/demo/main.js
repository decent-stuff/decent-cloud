import { DecentCloudClient, DecentCloudLedger } from '@decent-stuff/dc-client';

/**
 * Display a string in the specified element
 * @param {string} elementId The ID of the element to display the string in
 * @param {string} string The string to display
 * @param {boolean} error Whether the string is an error message
 */
function displayString(elementId, string, error = false) {
  const output = document.getElementById(elementId);
  if (error) {
    output.style.color = '#dc3545';
  } else {
    output.style.color = '#000000';
  }
  output.textContent = string;
}

/**
 * Display a JSON object in the specified element
 * @param {string} elementId The ID of the element to display the JSON in
 * @param {object} data The JSON object to display
 * @param {boolean} error Whether the JSON is an error message
 */
function displayJSON(elementId, data, error = false) {
  const output = document.getElementById(elementId);
  if (error) {
    output.style.color = '#dc3545';
  } else {
    output.style.color = '#000000';
  }
  output.textContent = JSON.stringify(
    data,
    (key, value) => {
      if (typeof value === 'bigint') {
        return Number(value);
      }
      return value;
    },
    2
  );
}

/**
 * Class-based demo implementation
 * This class demonstrates how to use the DecentCloudClient class
 */
class DecentCloudDemo {
  constructor() {
    this.client = null;
    this.initialized = false;
  }

  /**
   * Initialize the client
   */
  async initialize() {
    try {
      displayString('initStatus', 'Initializing client...');

      // Create a client instance
      this.client = new DecentCloudClient();

      console.log('Client created');

      // Initialize the client
      await this.client.initialize();

      this.initialized = true;
      displayString('initStatus', 'Client initialized successfully');

      return true;
    } catch (error) {
      console.error('Error initializing client:', error);
      displayString('initStatus', `Error initializing client: ${error.message}`, true);
      return false;
    }
  }

  /**
   * Fetch ledger blocks
   */
  async fetchBlocks() {
    if (!this.initialized) {
      displayString('fetchStatus', 'Client not initialized', true);
      return false;
    }

    try {
      displayString('fetchStatus', 'Fetching ledger blocks...');

      // Fetch ledger blocks
      const newBlocksCount = await DecentCloudLedger.fetchLedgerBlocks();

      displayString('fetchStatus', `Fetched ${newBlocksCount} new blocks`);

      return newBlocksCount;
    } catch (error) {
      console.error('Error fetching blocks:', error);
      displayString('fetchStatus', `Error fetching blocks: ${error.message}`, true);
      return false;
    }
  }

  /**
   * Display the last fetched block
   */
  async displayLastBlock() {
    if (!this.initialized) {
      displayString('wasmBlockHeader', 'Client not initialized', true);
      displayJSON('wasmBlockContents', { error: 'Client not initialized' }, true);
      return false;
    }

    try {
      // Get the last fetched block
      const lastEntry = await DecentCloudLedger.getLastFetchedBlock();

      if (lastEntry) {
        displayString('wasmBlockHeader', 'Last fetched block:');
        displayJSON('wasmBlockContents', lastEntry.ledgerEntry);
        return true;
      } else {
        displayString('wasmBlockHeader', 'No blocks fetched yet');
        displayJSON('wasmBlockContents', { message: 'No blocks fetched yet' });
        return false;
      }
    } catch (error) {
      console.error('Error displaying last block:', error);
      displayString('wasmBlockHeader', `Error displaying last block: ${error.message}`, true);
      displayJSON('wasmBlockContents', { error: error.message }, true);
      return false;
    }
  }

  /**
   * Clear the ledger storage
   */
  async clearStorage() {
    if (!this.initialized) {
      displayString('clearStatus', 'Client not initialized', true);
      return false;
    }

    try {
      displayString('clearStatus', 'Clearing ledger storage...');

      // Clear the ledger storage
      await DecentCloudLedger.clearStorage();

      displayString('clearStatus', 'Ledger storage cleared successfully');

      // Update the display
      displayString('wasmBlockHeader', 'No blocks fetched yet');
      displayJSON('wasmBlockContents', { message: 'No blocks fetched yet' });

      return true;
    } catch (error) {
      console.error('Error clearing storage:', error);
      displayString('clearStatus', `Error clearing storage: ${error.message}`, true);
      return false;
    }
  }
}

/**
 * Initialize the demo
 * This function is called when the page loads
 */
async function initDemo() {
  console.log('Initializing class-based demo...');

  // Create a demo instance
  const demo = new DecentCloudDemo();

  // Initialize the client
  await demo.initialize();

  // Set up event listeners for the buttons
  document.getElementById('fetchButton').addEventListener('click', async () => {
    await demo.fetchBlocks();
    await demo.displayLastBlock();
  });

  document.getElementById('clearButton').addEventListener('click', async () => {
    await demo.clearStorage();
  });

  // Display the last block if available
  await demo.displayLastBlock();
}

// Initialize the demo when the page loads
document.addEventListener('DOMContentLoaded', initDemo);
