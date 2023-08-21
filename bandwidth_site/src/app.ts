import init, { LatencyClient } from '../wasm/wasm_client.js';

function setSpanText(id: string, text: string): void {
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

function latencyUrl() : string {
    let url = "";
    const currentUrlWithoutAnchors = window.location.href.split('#')[0].replace("https://", "").replace("http://", "");
    if (window.location.href.startsWith("https://")) {
        url = "wss://" + currentUrlWithoutAnchors + "ws";
    } else {
        url = "ws://" + currentUrlWithoutAnchors + "ws";
    }
    return url;
}

declare global {
    interface Window {
        reportLatency: typeof reportLatency,
        latencyClient: LatencyClient,
    }
}
window.reportLatency = reportLatency;

// Load the WASM Module
await init();
console.log("WASM Loaded");

// Connect
let latencyClient = new LatencyClient(latencyUrl());
window.latencyClient = latencyClient;
window.latencyClient.connect_socket();

// Loop
window.setInterval(() => {
    if (window.latencyClient.is_connected()) {
        window.latencyClient.start_latency_run();
    }
}, 1000);