# Web-Bandwidth

Placeholder project.

## Concept

A webserver that supports websockets. The server can:

* Provide chunks of a fixed size for timing.
* Receive chunks of a fix size for timing.
* Perform sub-MTU exchange of timestamps to approximate latency.

A WASM-enabled client site that can:

* Run a simple speed-test (one way), timing download and/or upload of chunks.
* Run up/down concurrent tests.
* Measure latency with and without load.
* Assemble the results.

General:

* Entirely open source.
* Open about methodology.
* Open to improvements and contributions.

## Inspiration list:

* https://ankitbko.github.io/blog/2022/06/websocket-latency/


## Project Structure

* `bandwidth_server` - an Axum/Tokio Rust server that hosts the tests.
* `shared_data` - data structures that are shared between client and server, along with helper functions to use them.
* `wasm_client` - a WebAssembly client designed to run in the browser. Not stand-alone.
* `bandwidth_site` - (Not yet implemented) A Typescript site designed to be server from the bandwidth server, provide the client to the end-user's browser, and display the results.