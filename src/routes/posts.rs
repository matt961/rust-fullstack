use std::sync::Arc;

use anyhow::anyhow;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, Request, State};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{extract::ws::Message, routing::get};
use axum::{RequestExt, Router};
use futures_util::{SinkExt, StreamExt};
use tracing::{error, info, Instrument};

use crate::background::posts_broker::PostsSubscriptionManager;
use crate::error::AppError;

type PostsRouteState = (Arc<PostsSubscriptionManager>, deadpool_lapin::Pool);

async fn ws(
    State((sub_mgr, _)): State<PostsRouteState>,
    Path(path): Path<String>,
    request: Request,
) -> impl IntoResponse {
    request
        .extract::<WebSocketUpgrade, _>()
        .await
        .map(|wsu| {
            wsu.on_failed_upgrade(|e| {
                error!(target: "ahh", "ws upgrade failed: {:?}", e);
            })
            .on_upgrade(|ws| {
                async move {
                    let (mut sink, _stream) = ws.split();

                    let subscription = sub_mgr.subscribe();
                    let mut stream = tokio_stream::wrappers::ReceiverStream::from(subscription);

                    while let Some(x) = stream.next().await {
                        match sink
                            .feed(Message::Binary(
                                serde_json::to_string(x.as_ref())
                                    .unwrap_or("{}".to_owned())
                                    .into_bytes(),
                            ))
                            .await
                        {
                            Ok(_) => (),
                            Err(e) => {
                                error!({ path }, "no work {}", anyhow!(e));
                            }
                        }
                    }

                    info!("done sending posts");
                }
                .in_current_span()
            })
        })
        .map_err(AppError::from)
}

async fn create_post(
    State((_, rmq_conn)): State<PostsRouteState>,
    req: Request,
) -> impl IntoResponse {
    let form: Form = req.extract();
}

pub fn router() -> Router<PostsRouteState> {
    Router::new()
        .route("/ws", get(ws))
        .route("/", post(create_post))
}
