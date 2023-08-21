//! WebAssembly Client. Designed to be loaded as part of the embedded
//! website, rather than used standalone.

use shared_data::{LatencyTest, MAGIC_NUMBER};
use thiserror::Error;
use wasm_bindgen::prelude::*;
use web_sys::{BinaryType, ErrorEvent, MessageEvent, WebSocket};

static mut CONDUIT: Option<Conduit> = None;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn initialize_wss(url: String) {
    log(&format!("Initializing WSS to: {url}"));
    unsafe {
        if CONDUIT.is_none() {
            CONDUIT = Some(Conduit::new(url));

            if let Some(conduit) = &mut CONDUIT {
                match conduit.connect() {
                    Ok(_) => log("Connection requested."),
                    Err(e) => log(&format!("Error connecting: {:?}", e)),
                }
            }
        } else {
            log("Conduit already initialized");
        }
    }
}

#[wasm_bindgen]
pub fn is_wasm_connected() -> bool {
    unsafe {
        if let Some(conduit) = &CONDUIT {
            conduit.is_connected()
        } else {
            false
        }
    }
}

#[wasm_bindgen]
pub fn start_latency_run() {
    unsafe {
        if let Some(conduit) = &mut CONDUIT {
            if conduit.is_connected() {
                log("Starting Latency Run");
                let bytes = LatencyTest::InitialRequest {
                    magic: MAGIC_NUMBER,
                }
                .encode();
                if let Some(socket) = &mut conduit.socket {
                    socket.send_with_u8_array(&bytes).unwrap();
                }
            } else {
                log("Not connected");
            }
        } else {
            log("Not initialized");
        }
    }
}

pub fn unix_now_ms() -> u128 {
    use web_time::SystemTime;
    match SystemTime::now().duration_since(web_time::UNIX_EPOCH) {
        Ok(t) => t.as_millis(),
        Err(_e) => 0,
    }
}

#[derive(Error, Debug)]
enum WebSocketError {
    #[error("URL is empty")]
    NoURL,
    #[error("Already connected")]
    AlreadyConnected,
    #[error("WebSocket already exists")]
    AlreadyExists,
    #[error("WebSocket Creation Error")]
    CreationError,
}

#[derive(PartialEq, Eq)]
enum ConnectionStatus {
    New,
    Connected,
}

/// Handles WS connection to the server.
struct Conduit {
    status: ConnectionStatus,
    socket: Option<WebSocket>,
    url: String,
}

impl Conduit {
    fn new(url: String) -> Self {
        Self {
            status: ConnectionStatus::New,
            socket: None,
            url,
        }
    }

    fn connect(&mut self) -> Result<(), WebSocketError> {
        // Precondition testing
        if self.url.is_empty() {
            return Err(WebSocketError::NoURL);
        }
        if self.status != ConnectionStatus::New {
            return Err(WebSocketError::AlreadyConnected);
        }
        if self.socket.is_some() {
            return Err(WebSocketError::AlreadyExists);
        }
        log(&format!("Connecting to: {}", self.url));
        let conn_result = WebSocket::new(&self.url);
        if conn_result.is_err() {
            log(&format!("Error connecting: {:?}", conn_result));
            return Err(WebSocketError::CreationError);
        }
        self.socket = Some(conn_result.unwrap());
        if let Some(socket) = &mut self.socket {
            socket.set_binary_type(BinaryType::Arraybuffer);

            // Wire up on_close
            let onclose_callback = Closure::<dyn FnMut(_)>::new(move |_e: ErrorEvent| {
                on_close();
            });
            socket.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
            onclose_callback.forget();

            // Wire up on_error
            let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
                log(&format!("Error Received: {e:?}"));
                on_error()
            });
            socket.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
            onerror_callback.forget();

            // Wire up on_open
            let onopen_callback = Closure::<dyn FnMut(_)>::new(move |_e: ErrorEvent| {
                //log("Open Received");
                on_open();
            });
            socket.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
            onopen_callback.forget();

            // Wire up on message
            let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
                log("Message Received");
                if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                    let array = js_sys::Uint8Array::new(&abuf);
                    let raw = array.to_vec();
                    let decoded = LatencyTest::decode(&raw).unwrap();
                    match decoded {
                        LatencyTest::FirstReply { magic, server_time } => {
                            assert_eq!(magic, MAGIC_NUMBER);
                            let reply = LatencyTest::FirstResponse {
                                magic: MAGIC_NUMBER,
                                server_time,
                                client_time: unix_now_ms(),
                            };
                            unsafe {
                                if let Some(socket) = &mut CONDUIT.as_mut().unwrap().socket {
                                    socket.send_with_u8_array(&reply.encode()).unwrap();
                                }
                            }
                        }
                        LatencyTest::SecondReply { magic, server_time, client_time, server_ack_time } => {
                            assert_eq!(magic, MAGIC_NUMBER);                            
                            let final_result = LatencyTest::Final {
                                magic: MAGIC_NUMBER,
                                server_time,
                                client_time,
                                server_ack_time,
                                client_ack_time: unix_now_ms(),
                            };
                            let (average, server, client) = final_result.calculate_latency();
                            log(&format!("Average: {}ms, Server: {}ms, Client: {}ms", average, server, client));
                        }
                        _ => {
                            log(&format!("Received: {:?}", decoded));
                        }
                    }
                }
            });
            socket.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
            onmessage_callback.forget();
        }

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.status == ConnectionStatus::Connected
    }
}

fn on_close() {
    unsafe {
        if let Some(conduit) = &mut CONDUIT {
            conduit.socket = None;
            conduit.status = ConnectionStatus::New;
        }
    }
}

fn on_error() {
    unsafe {
        if let Some(conduit) = &mut CONDUIT {
            conduit.socket = None;
            conduit.status = ConnectionStatus::New;
        }
    }
}

fn on_open() {
    unsafe {
        if let Some(conduit) = &mut CONDUIT {
            conduit.status = ConnectionStatus::Connected;
        }
    }
}
