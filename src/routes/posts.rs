use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Router;
use axum::{extract::ws::Message, routing::get};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tracing::{error, Instrument};

async fn ws(
    State((rabbit_pool,)): State<(deadpool_lapin::Pool,)>,
    wsu: WebSocketUpgrade,
) -> impl IntoResponse {
    wsu.on_failed_upgrade(|e| {
        error!(target: "ahh", "ws upgrade failed: {:?}", e);
    })
    .on_upgrade(|ws| {
        async move {
            let (mut sink, _stream) = ws.split();

            let _ = sink
                .feed(Message::Binary(
                    serde_json::to_string(&json!({}))
                        .unwrap_or("{}".to_owned())
                        .into_bytes(),
                ))
                .in_current_span()
                .await;
        }
        .in_current_span()
    })
}

pub fn router() -> Router<(deadpool_lapin::Pool,)> {
    Router::new().route("/ws", get(ws))
}
