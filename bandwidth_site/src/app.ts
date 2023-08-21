import init, { LatencyClient } from '../wasm/wasm_client.js';

const N_BANDS = 20;
const BAND_DIVISOR = 10.0;

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

    if (avg > window.worst) {
        window.worst = avg;
        setSpanText("worstLatency", avg.toString() + " ms");
    }
    if (avg < window.best) {
        window.best = avg;
        setSpanText("bestLatency", avg.toString() + " ms");
    }

    let bin = Math.floor(avg.valueOf() / BAND_DIVISOR);
    window.frequency[bin] += 1;

    let html = "<table border='0'>";
    for (let i=0; i<N_BANDS; i++) {
        html += "<tr><td>" + (i * BAND_DIVISOR) + " - " + ((i+1) * BAND_DIVISOR) + "</td><td>" + window.frequency[i] + "</td></tr>";
    }
    html += "</table>";
    let target = document.getElementById("histo");
    if (target) {
        target.innerHTML = html;
    }
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
        worst: Number,
        best: Number,
        frequency: number[],
    }
}
window.reportLatency = reportLatency;
window.worst = 0;
window.best = 10000;
window.frequency = [];
for (let i=0; i<N_BANDS; i++) {
    window.frequency.push(0);
}

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