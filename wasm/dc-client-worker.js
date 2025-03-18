/**
 * Shared Worker for Decent Cloud WASM module.
 * This worker enables sharing a single WASM instance across multiple browser tabs.
 */

// Shared state across all connections
let wasmInitialized = false;
const connections = [];

// Handle new connections
self.onconnect = function (e) {
  const port = e.ports[0];
  connections.push(port);

  port.onmessage = function (event) {
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
