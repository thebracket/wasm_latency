import init, { initialize_wss, is_wasm_connected, start_latency_run } from '../wasm/wasm_client.js';

// Load the WASM Module
await init();
console.log("WASM Loaded");

// Connect
initialize_wss("ws://localhost:3000/ws");

// Loop
window.setInterval(() => {
    if (is_wasm_connected()) {
        console.log("WASM Connected");
        start_latency_run();
    }
}, 1000);