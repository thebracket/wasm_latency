use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::{response::IntoResponse, routing::get, Router};
use shared_data::LatencyTest;
use tokio::sync::mpsc::Sender;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(index));
    //.route("/ws", get(ws_handler));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn index() -> &'static str {
    "Hello World"
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
        LatencyTest::FirstResponse { magic, server_time, client_time } => {
            assert_eq!(magic, shared_data::MAGIC_NUMBER);
            let reply = LatencyTest::SecondReply { magic, server_time, client_time, server_ack_time: shared_data::unix_now_ms() };
            tx.send(reply.encode()).await.unwrap();
        }
        _ => {
            tracing::warn!("Message not expected by server: {decoded:?}");
        }
    }
}