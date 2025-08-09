mod types;
mod user_manager;
mod redis_manager;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    routing::get,
    response::IntoResponse,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use tokio::signal;
use tracing::{error, info};
use user_manager::UserManager;
use uuid::Uuid;

use crate::types::{ClientMsg, Feed, ServerMsg};

#[derive(Clone)]
struct AppState {
    users: UserManager,
    redis_url: Arc<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
    let state = AppState {
        users: UserManager::default(),
        redis_url: Arc::new(redis_url.clone()),
    };

    // start redis listener
    {
        let users = state.users.clone();
        let url = redis_url.clone();
        tokio::spawn(async move {
            if let Err(e) = redis_manager::run_redis_listener(&url, users).await {
                error!("Redis listener error: {e}");
            }
        });
    }

    // ws route
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr: SocketAddr = "0.0.0.0:9000".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await
        .unwrap();
}

async fn shutdown() {
    let _ = signal::ctrl_c().await;
    tracing::info!("Shutting down WS server...");
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| client_socket(socket, state))
}

async fn client_socket(mut socket: WebSocket, state: AppState) {
    // register new client
    let (client_id, mut rx) = state.users.register_client().await;
    let (mut ws_tx, mut ws_rx) = socket.split();

    // writer task: forward ServerMsg from mpsc to websocket
    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let txt = serde_json::to_string(&msg).unwrap_or_else(|_| "{\"type\":\"info\",\"message\":\"encode error\"}".to_string());
            if ws_tx.send(Message::Text(txt)).await.is_err() {
                break;
            }
        }
    });

    // reader loop: process client messages
    let users = state.users.clone();
    let reader = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(txt) => {
                    let parsed = serde_json::from_str::<ClientMsg>(&txt);
                    match parsed {
                        Ok(ClientMsg::Ping) => {
                            let _ = users.broadcast((Feed::Ticker, Uuid::nil()), ServerMsg::Pong).await;
                        }
                        Ok(ClientMsg::Subscribe { market_id, feeds }) => {
                            for f in feeds.iter().copied() {
                                users.subscribe(client_id, (f, market_id)).await;
                            }
                            let _ = users.inner.read().await.clients.get(&client_id)
                                .map(|tx| tx.try_send(ServerMsg::Info{ message: format!("subscribed to {feeds:?} {market_id}")}));
                        }
                        Ok(ClientMsg::Unsubscribe { market_id, feeds }) => {
                            for f in feeds.iter().copied() {
                                users.unsubscribe(client_id, (f, market_id)).await;
                            }
                            let _ = users.inner.read().await.clients.get(&client_id)
                                .map(|tx| tx.try_send(ServerMsg::Info{ message: format!("unsubscribed from {feeds:?} {market_id}")}));
                        }
                        Err(e) => {
                            if let Some(tx) = users.inner.read().await.clients.get(&client_id) {
                                let _ = tx.try_send(ServerMsg::Info { message: format!("invalid message: {e}") });
                            }
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Binary(_) | Message::Ping(_) | Message::Pong(_) => {} // ignore
            }
        }
    });

    // wait for either to finish
    let _ = tokio::join!(writer, reader);

    // cleanup
    state.users.unregister_client(client_id).await;
}