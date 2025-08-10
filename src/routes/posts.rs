use std::sync::Arc;

use anyhow::anyhow;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Request, State};
use axum::response::{Html, IntoResponse};
use axum::routing::post;
use axum::{Form, RequestExt, Router};
use axum::{extract::ws::Message, routing::get};
use bytes::Bytes;
use futures_util::StreamExt;
use lapin::BasicProperties;
use lapin::options::BasicPublishOptions;
use macros::ert;
use tera::Tera;
use tokio::sync::RwLock;
use tracing::{Span, error, info, warn};

use crate::background::posts_broker::PostsSubscriptionManager;
use crate::error::AppError;
use crate::models::post::Post;
use crate::services::Pool;

type PostsRouteState = (
    Arc<RwLock<Tera>>,
    Arc<PostsSubscriptionManager>,
    deadpool_lapin::Pool,
    Pool,
);

async fn ws(
    State((tera, sub_mgr, _, _)): State<PostsRouteState>,
    wsu: WebSocketUpgrade,
) -> axum::response::Result<impl IntoResponse> {
    info!("ahhhh");
    let s = Span::current();
    info!("span id: {:?}", s.id());
    let res = wsu
        .on_failed_upgrade(|e| {
            error!(target: "ahh", "ws upgrade failed: {:?}", e);
        })
        .on_upgrade(|mut ws| async move {
            info!("new ws conn");

            let subscription = sub_mgr.subscribe();
            let id = subscription.id;
            let mut stream = tokio_stream::wrappers::ReceiverStream::from(subscription.rx);

            while let Some(x) = stream.next().await {
                info!("new post");
                let mut ctx = tera::Context::new();
                ctx.insert("post", x.as_ref());
                let html = tera
                    .read()
                    .await
                    .render("posts/ws_post.html", &ctx)
                    .inspect_err(ert!())
                    .unwrap_or_default();
                if let Err(e) = ws.send(Message::Ping(Bytes::from_static(b"foo"))).await {
                    warn!(%e, "ws ping failed");
                    continue;
                }
                match ws.send(Message::Text(html.into())).await {
                    Ok(_) => (),
                    Err(e) => {
                        warn!(%e, "ws died");
                        let _ = sub_mgr
                            .unsubscribe(&id)
                            .ok_or_else(|| anyhow!("already unsubscribed: {}", &id))
                            .inspect_err(ert!());
                        return;
                    }
                };
            }

            info!("done sending posts");
        });
    Ok(res)
}

#[tracing::instrument(skip_all)]
async fn create_post(
    State((tera, _, rmq_conn_pool, db_pool)): State<PostsRouteState>,
    req: Request,
) -> axum::response::Result<Html<Bytes>> {
    use crate::models::post::CreatePost;
    use crate::schema::posts::dsl::*;
    use diesel_async::RunQueryDsl;

    let Form(f): Form<CreatePost> = req.extract().await.map_err(AppError::from)?;
    let mut conn = db_pool.get().await.map_err(AppError::from)?;

    let post = diesel::insert_into(posts)
        .values(f)
        .get_result::<Post>(&mut conn)
        .await
        .map_err(AppError::from)?;

    let channel = rmq_conn_pool
        .get()
        .await
        .map_err(AppError::from)
        .inspect_err(ert!())?
        .create_channel()
        .await
        .map_err(AppError::from)
        .inspect_err(ert!())?;
    channel
        .basic_publish(
            "",
            "posts",
            BasicPublishOptions {
                mandatory: true,
                ..Default::default()
            },
            serde_json::to_vec(&post)
                .inspect_err(ert!())
                .map_err(AppError::from)?
                .as_slice(),
            BasicProperties::default().with_content_type("application/json".into()),
        )
        .await
        .map_err(AppError::from)
        .inspect_err(ert!())?
        .await
        .inspect_err(ert!())
        .map_err(|e| Html(e.to_string()))?;

    let teractx =
        tera::Context::from_value(serde_json::json!({"post": post})).map_err(|e| AppError {
            inner: anyhow!("could not template response: {}", e),
        })?;

    let body = tera
        .read()
        .await
        .render("posts/create_post.html", &teractx)
        .inspect_err(ert!())
        .map_err(AppError::from)?;
    Ok(Html(Bytes::from(body)))
}

pub fn router() -> Router<PostsRouteState> {
    Router::new()
        .route("/ws", get(ws))
        .route("/", post(create_post))
}
