import { DecentCloudClient, decentCloudLedger, db } from '@decent-stuff/dc-client';

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
 * Check for database errors and display them if present
 * @returns {boolean} Whether an error was displayed
 */
function checkAndDisplayDatabaseError() {
  const error = db.getError();
  const errorDiv = document.querySelector('.status[style*="background-color: #ffeeee"]');

  if (error) {
    displayString('dbErrorStatus', `${error}`, true);
    errorDiv.style.display = 'block';
    return true;
  } else {
    displayString('dbErrorStatus', '');
    errorDiv.style.display = 'none';
    return false;
  }
}

/**
 * This class demonstrates how to use DecentCloudClient
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

      // Check for database errors
      if (checkAndDisplayDatabaseError()) {
        displayString('initStatus', 'Client initialized with database errors', true);
      } else {
        this.initialized = true;
      }
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
      const fetchStatus = await decentCloudLedger.fetchLedgerBlocks();

      // Check for database errors
      if (checkAndDisplayDatabaseError()) {
        displayString('fetchStatus', 'Error fetching blocks. See error details above.', true);
      } else {
        displayString('fetchStatus', fetchStatus);
      }
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
      // Check for database errors first
      if (checkAndDisplayDatabaseError()) {
        displayString('wasmBlockHeader', 'Error displaying block. See error details above.', true);
        displayJSON('wasmBlockContents', { error: 'Database error occurred' }, true);
        return false;
      }

      // Get the last fetched block
      const lastBlock = await decentCloudLedger.getLastFetchedBlock();

      // Check for database errors again after fetching the block
      if (checkAndDisplayDatabaseError()) {
        displayString('wasmBlockHeader', 'Error displaying block. See error details above.', true);
        displayJSON('wasmBlockContents', { error: 'Database error occurred' }, true);
        return false;
      }

      if (lastBlock) {
        const lastBlockEntries = await decentCloudLedger.getBlockEntries(lastBlock.blockOffset);

        // Check for database errors after fetching block entries
        if (checkAndDisplayDatabaseError()) {
          displayString(
            'wasmBlockHeader',
            'Error displaying block entries. See error details above.',
            true
          );
          displayJSON('wasmBlockContents', { error: 'Database error occurred' }, true);
          return false;
        }

        displayJSON('wasmBlockHeader', lastBlock);
        displayJSON('wasmBlockContents', lastBlockEntries);
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
      await decentCloudLedger.clearStorage();

      // Check for database errors
      if (checkAndDisplayDatabaseError()) {
        displayString('clearStatus', 'Error clearing storage. See error details above.', true);
      } else {
        displayString('clearStatus', 'Ledger storage cleared successfully');
      }

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
