use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::{response::IntoResponse, routing::get, Router};
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
                        
                        /*tokio::spawn(
                            handle_socket_message(msg, tx.clone())
                        );*/
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
                    Some(msg) => {
                        todo!();
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
