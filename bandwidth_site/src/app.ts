import init, { initialize_wss, is_wasm_connected, start_latency_run } from '../wasm/wasm_client.js';

function setSpanText(id: string, text: string) : void {
    let item = document.getElementById(id);
    if (item) {
        item.innerText = text;
    } else {
        console.error("Could not find element with id: " + id);
    }
}

function reportLatency(avg: Number, server: Number, client: Number) {
    setSpanText("averageLatency", avg.toString() + " ms");
    setSpanText("clientLatency", client.toString() + " ms");
    setSpanText("serverLatency", server.toString() + " ms");
}

declare global {
    interface Window {
        reportLatency: typeof reportLatency,
    }
}
window.reportLatency = reportLatency;

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