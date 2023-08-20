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


