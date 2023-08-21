use axum::body::StreamBody;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::http::{HeaderMap, header};
use axum::response::Html;
use axum::{response::IntoResponse, routing::get, Router};
use shared_data::LatencyTest;
use tokio_util::io::ReaderStream;
use tracing_subscriber::fmt::format::FmtSpan;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;

#[tokio::main]
async fn main() {
    // Start the logger
    set_console_logging().unwrap();

    // Start the webserver
    let app = Router::new()
        .route("/", get(index_page))
        .route("/app.js", get(js_bundle))
        .route("/app.js.map", get(js_map))
        .route("/style.css", get(css))
        .route("/style.css.map", get(css_map))
        .route("/wasm_client_bg.wasm", get(wasm_file))
        .route("/ws", get(ws_handler));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn set_console_logging() -> anyhow::Result<()> {
    // install global collector configured based on RUST_LOG env var.
    let subscriber = tracing_subscriber::fmt()
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Display the thread ID an event was recorded on
        .with_thread_ids(true)
        // Don't display the event's target (module path)
        .with_target(false)
        // Include per-span timings
        .with_span_events(FmtSpan::CLOSE)
        // Build the subscriber
        .finish();

    // Set the subscriber as the default
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

const JS_BUNDLE: &str = include_str!("../../bandwidth_site/out/app.js");
const JS_MAP: &str = include_str!("../../bandwidth_site/out/app.js.map");
const CSS: &str = include_str!("../../bandwidth_site/out/style.css");
const CSS_MAP: &str = include_str!("../../bandwidth_site/out/style.css.map");
const HTML_MAIN: &str = include_str!("../../bandwidth_site/src/main.html");
const WASM_BODY: &[u8] = include_bytes!("../../bandwidth_site/wasm/wasm_client_bg.wasm");

async fn index_page() -> Html<String> {
    Html(HTML_MAIN.to_string())
}

async fn js_bundle() -> axum::response::Response<String> {
    axum::response::Response::builder()
        .header("Content-Type", "text/javascript")
        .body(JS_BUNDLE.to_string())
        .unwrap()
}

async fn js_map() -> axum::response::Response<String> {
    axum::response::Response::builder()
        .header("Content-Type", "text/json")
        .body(JS_MAP.to_string())
        .unwrap()
}

async fn css() -> axum::response::Response<String> {
    axum::response::Response::builder()
        .header("Content-Type", "text/css")
        .body(CSS.to_string())
        .unwrap()
}

async fn css_map() -> axum::response::Response<String> {
    axum::response::Response::builder()
        .header("Content-Type", "text/json")
        .body(CSS_MAP.to_string())
        .unwrap()
}

async fn wasm_file() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/wasm"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_static("attachment; filename=wasm_pipe_bg.wasm"),
    );
    axum::response::Response::builder()
        .header(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/wasm"),
        )
        .header(
            header::CONTENT_DISPOSITION,
            header::HeaderValue::from_static("attachment; filename=wasm_pipe_bg.wasm"),
        )
        .body(StreamBody::new(ReaderStream::new(WASM_BODY)))
        .unwrap()
}


pub async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    tracing::info!("WS Upgrade Called");
    ws.on_upgrade(move |sock| handle_socket(sock))
}

async fn handle_socket(mut socket: WebSocket) {
    tracing::info!("WebSocket Connected");

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(10);

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Binary(bytes))) => {
                        // Spawn a new task, so we keep trucking in the meantime
                        tokio::spawn(
                            handle_socket_message(bytes, tx.clone())
                        );
                    }
                    Some(Err(e)) => {
                        tracing::error!("Error receiving message: {:?}", e);
                        //break;
                    }
                    None => {
                        tracing::info!("WebSocket Disconnected");
                        break;
                    }
                    _ => {
                        tracing::error!("Message in non-binary format");
                        break;
                    }
                }
            },
            msg = rx.recv() => {
                match msg {
                    Some(bytes) => {
                        socket.send(Message::Binary(bytes)).await.unwrap();
                    }
                    None => {
                        tracing::info!("WebSocket Disconnected");
                        break;
                    }
                }
            },
        }
    }
}

async fn handle_socket_message(bytes: Vec<u8>, tx: Sender<Vec<u8>>) {
    let decoded = LatencyTest::decode(&bytes).unwrap();
    match decoded {
        LatencyTest::InitialRequest { magic } => {
            assert_eq!(magic, shared_data::MAGIC_NUMBER);
            let reply = LatencyTest::FirstReply {
                magic: shared_data::MAGIC_NUMBER,
                server_time: shared_data::unix_now_ms(),
            };
            tx.send(reply.encode()).await.unwrap();
        }
        LatencyTest::FirstResponse {
            magic,
            server_time,
            client_time,
        } => {
            assert_eq!(magic, shared_data::MAGIC_NUMBER);
            let reply = LatencyTest::SecondReply {
                magic,
                server_time,
                client_time,
                server_ack_time: shared_data::unix_now_ms(),
            };
            tx.send(reply.encode()).await.unwrap();
        }
        _ => {
            tracing::warn!("Message not expected by server: {decoded:?}");
        }
    }
}
