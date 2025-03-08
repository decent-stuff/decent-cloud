import { createClient } from '../dist/dc-client.js';

function displayString(elementId, string, error = false) {
  const output = document.getElementById(elementId);
  if (error) {
    output.style.color = '#dc3545';
  } else {
    output.style.color = '#000000';
  }
  output.textContent = string;
}

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

// Initialize the demo
async function initDemo() {
  console.log('Initializing demo...');

  // Create a client instance
  const client = createClient();

  // Initialize the client
  await client.initialize();

  // Get block as JSON using the new class-based API
  const blockResult = client.ledger.getBlockAsJson(BigInt(0));
  console.log('Block result type:', typeof blockResult, blockResult);

  try {
    // Parse the result if it's a string
    let parsedResult;
    if (typeof blockResult === 'string') {
      parsedResult = JSON.parse(blockResult);
    } else {
      parsedResult = blockResult;
    }

    // Extract the block header and contents
    const block_header = parsedResult.block_header;
    const ledger_block = parsedResult.block;

    displayJSON('wasmBlockHeader', block_header);
    displayJSON('wasmBlockContents', ledger_block);
  } catch (error) {
    console.error('Error processing block result:', error, blockResult);
    displayJSON(
      'wasmBlockHeader',
      { error: 'Failed to process block header: ' + error.message },
      true
    );
    displayJSON(
      'wasmBlockContents',
      { error: 'Failed to process block contents: ' + error.message },
      true
    );
  }
}

// Initialize the demo when the page loads
document.addEventListener('DOMContentLoaded', initDemo);
