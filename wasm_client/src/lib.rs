//! WebAssembly Client. Designed to be loaded as part of the embedded
//! website, rather than used standalone.

use std::{cell::RefCell, rc::Rc};
use shared_data::{LatencyTest, MAGIC_NUMBER, unix_now_ms};
use thiserror::Error;
use wasm_bindgen::prelude::*;
use web_sys::{BinaryType, ErrorEvent, MessageEvent, WebSocket};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_name = "window.reportLatency")]
    fn report_latency(average: f64, server: f64, client: f64);
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
#[wasm_bindgen]
pub struct LatencyClient {
    inner: Rc<RefCell<LatencyClientInner>>,
}

struct LatencyClientInner {
    status: ConnectionStatus,
    socket: Option<WebSocket>,
    url: String,
}

#[wasm_bindgen]
impl LatencyClient {
    #[wasm_bindgen(constructor)]
    pub fn new(url: String) -> Self {
        Self {
            inner: Rc::new(RefCell::new(LatencyClientInner {
                status: ConnectionStatus::New,
                socket: None,
                url,
            })),
        }
    }

    #[wasm_bindgen]
    pub fn connect_socket(&mut self) {
        match self.connect() {
            Ok(_) => log("Connection requested."),
            Err(e) => log(&format!("Error connecting: {:?}", e)),
        }
    }

    fn connect(&mut self) -> Result<(), WebSocketError> {
        // Precondition testing
        if self.inner.borrow().url.is_empty() {
            return Err(WebSocketError::NoURL);
        }
        if self.inner.borrow().status != ConnectionStatus::New {
            return Err(WebSocketError::AlreadyConnected);
        }
        if self.inner.borrow().socket.is_some() {
            return Err(WebSocketError::AlreadyExists);
        }
        log(&format!("Connecting to: {}", self.inner.borrow().url));
        let conn_result = WebSocket::new(&self.inner.borrow().url);
        if conn_result.is_err() {
            log(&format!("Error connecting: {:?}", conn_result));
            return Err(WebSocketError::CreationError);
        }
        self.inner.borrow_mut().socket = Some(conn_result.unwrap());
        if let Some(socket) = &self.inner.borrow().socket {
            socket.set_binary_type(BinaryType::Arraybuffer);

            // Wire up on_close
            let inner = self.inner.clone();
            let onclose_callback = Closure::<dyn FnMut(_)>::new(move |_e: ErrorEvent| {
                inner.borrow_mut().socket = None;
                inner.borrow_mut().status = ConnectionStatus::New;
            });
            socket.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
            onclose_callback.forget();

            // Wire up on_error
            let inner = self.inner.clone();
            let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
                log(&format!("Error Received: {e:?}"));
                inner.borrow_mut().socket = None;
                inner.borrow_mut().status = ConnectionStatus::New;
            });
            socket.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
            onerror_callback.forget();

            // Wire up on_open
            let inner = self.inner.clone();
            let onopen_callback = Closure::<dyn FnMut(_)>::new(move |_e: ErrorEvent| {
                //log("Open Received");
                inner.borrow_mut().status = ConnectionStatus::Connected;
            });
            socket.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
            onopen_callback.forget();

            // Wire up on message
            let onmsg_inner = self.inner.clone();
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
                            if let Some(socket) = &onmsg_inner.borrow().socket {
                                socket.send_with_u8_array(&reply.encode()).unwrap();
                            }
                        }
                        LatencyTest::SecondReply {
                            magic,
                            server_time,
                            client_time,
                            server_ack_time,
                        } => {
                            assert_eq!(magic, MAGIC_NUMBER);
                            let final_result = LatencyTest::Final {
                                magic: MAGIC_NUMBER,
                                server_time,
                                client_time,
                                server_ack_time,
                                client_ack_time: unix_now_ms(),
                            };
                            let (average, server, client) = final_result.calculate_latency();
                            log(&format!(
                                "Average: {}ms, Server: {}ms, Client: {}ms",
                                average, server, client
                            ));
                            report_latency(average, server, client);
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

    #[wasm_bindgen]
    pub fn is_connected(&self) -> bool {
        self.inner.borrow().status == ConnectionStatus::Connected
    }

    #[wasm_bindgen]
    pub fn start_latency_run(&self) {
        let bytes = LatencyTest::InitialRequest {
            magic: MAGIC_NUMBER,
        }
        .encode();
        if let Some(socket) = &self.inner.borrow().socket {
            socket.send_with_u8_array(&bytes).unwrap();
        }
    }
}
