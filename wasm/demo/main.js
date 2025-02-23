import { initialize, queryCanister, ledger_get_block_as_json } from '../dist/dc-client.js';

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
  try {
    console.log('Initializing demo...');
    // Initialize the WASM module

    // Set up event listeners
    // document.getElementById('checkWasm').addEventListener('click', checkStatus);

    const result = await initialize();
    console.info(`Initialization result: ${result}`);

    displayString('wasmInit', result);
    const result2 = JSON.parse(await ledger_get_block_as_json(BigInt(0)));
    const block_header = JSON.parse(result2['block_header']);
    const ledger_block = result2['block'];
    displayJSON('wasmBlockHeader', block_header);
    displayJSON('wasmBlockContents', ledger_block);
  } catch (error) {
    console.error('Failed to initialize demo:', error);
    displayJSON('wasmInit', `Initialization Error: ${error.message}`, true);
  }
}

// // Test basic functionality
// async function checkStatus() {
//   try {
//     // Try to get metadata as a simple query test
//     const metadata = await queryCanister('icrc1_metadata', []).catch(
//       err => `Metadata query failed: ${err.message}`
//     );

//     const status = {
//       initialized: true,
//       metadata,
//       timestamp: new Date().toISOString(),
//     };

//     displayOutput('wasmOutput', status);
//   } catch (error) {
//     console.error('Error in checkStatus:', error);
//     displayOutput('wasmOutput', `Error: ${error.message}`, true);
//   }
// }

// Initialize the demo when the page loads
document.addEventListener('DOMContentLoaded', initDemo);
