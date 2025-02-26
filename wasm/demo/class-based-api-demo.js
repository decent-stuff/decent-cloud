// Import the createClient function from the dc-client package
import { createClient } from '@decent-stuff/dc-client';

// Create a new DecentCloudClient instance with optional configuration
const client = createClient({
  networkUrl: 'https://icp-api.io',
  canisterId: 'ggi4a-wyaaa-aaaai-actqq-cai',
});

// Example usage of the class-based API
async function demoClassBasedApi() {
  try {
    // Initialize the client
    console.log('Initializing client...');
    const initResult = await client.initialize();
    console.log('Initialization result:', initResult);

    // Example: Using the storage API
    console.log('\n--- Storage API Examples ---');
    const storageSize = client.storage.sizeBytes();
    console.log('Storage size:', storageSize, 'bytes');

    // Example: Using the ledger API
    console.log('\n--- Ledger API Examples ---');
    const cursor = client.ledger.getCursorLocalAsString();
    console.log('Local cursor:', cursor);

    // Get a block as JSON
    console.log('\nFetching block as JSON...');
    const blockResult = client.ledger.getBlockAsJson(BigInt(0));
    console.log('Block result type:', typeof blockResult);

    try {
      const parsedResult = JSON.parse(blockResult);
      const block_header = JSON.parse(parsedResult.block_header);
      const block = parsedResult.block;
      console.log('Block header:', block_header);
      console.log('Block contents (first item):', block[0]);
    } catch (error) {
      console.error('Error parsing block result:', error);
    }

    // Get transactions
    console.log('\nFetching transactions...');
    const transactions = await client.ledger.getTransactions();
    console.log('Transactions:', transactions);

    // Example: Using the canister API
    console.log('\n--- Canister API Examples ---');
    console.log('Calling a query method...');
    const queryResult = await client.canister.callQuery('get_transactions', []);
    console.log('Query result:', queryResult);

    // Example: Working with ledger values
    console.log('\n--- Ledger Values Example ---');
    const label = 'example';
    const key = new TextEncoder().encode('test-key');
    const value = new TextEncoder().encode('test-value');

    // Set a value
    console.log('Setting a value in the ledger...');
    client.ledger.setValue(label, key, value);

    // Get the value back
    const retrievedValue = client.ledger.getValue(label, key);
    if (retrievedValue) {
      console.log('Retrieved value:', new TextDecoder().decode(retrievedValue));
    } else {
      console.log('Value not found');
    }

    // Remove the value
    console.log('Removing the value...');
    client.ledger.removeValue(label, key);

    // Verify it's gone
    const afterRemoval = client.ledger.getValue(label, key);
    console.log(
      'After removal:',
      afterRemoval === null ? 'Value was removed' : 'Value still exists'
    );
  } catch (error) {
    console.error('Error in demo:', error);
  }
}

// Run the demo
demoClassBasedApi();
