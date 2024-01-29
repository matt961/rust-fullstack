mod components;
mod config;
mod helpers;
mod middleware;
mod models;
mod routes;
mod schema;
mod services;

use std::error::Error;

use axum::Router;

use diesel_async::pooled_connection::deadpool::{Hook, Pool};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;

use figment::{providers::Format, Figment};

use helpers::error::AppError;
use services::users::UserServiceDb;
use tracing::*;
use tracing_subscriber::{prelude::*, EnvFilter};

use crate::middleware::logging::HttpLoggingExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cfg: config::DbCfg = Figment::new()
        .merge(figment::providers::Json::file("appsettings.json"))
        .merge(figment::providers::Env::prefixed("APP_"))
        .extract()?;

    // initialize tracing
    let fmtlayer = tracing_subscriber::fmt::layer();

    tracing_subscriber::registry()
        // .with_http_tracing()
        .with(fmtlayer)
        .with(EnvFilter::from_default_env())
        .init();

    // create a new connection pool with the default config
    let mgr =
        AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(&cfg.database_url);

    info!("Starting DB pool");
    let pool = Pool::builder(mgr)
        .max_size(10)
        .pre_recycle(Hook::async_fn(|conn, metrics| {
            tracing::info_span!("dbpool::pre_recycle").in_scope(|| {
                let c = std::ptr::addr_of!(conn);
                tracing::info!(?c, ?metrics, "Pre-recycle");
                Box::pin(std::future::ready(Ok(())))
            })
        }))
        .post_create(Hook::async_fn(|conn, metrics| {
            tracing::info_span!("dbpool::post_create").in_scope(|| {
                let c = std::ptr::addr_of!(conn);
                tracing::info!(?c, ?metrics, "Post-create");
                Box::pin(std::future::ready(Ok(())))
            })
        }))
        .runtime(deadpool::Runtime::Tokio1)
        .build()?;

    let user_svc = UserServiceDb::new(pool.clone());

    let app = Router::new()
        .nest_service("/", tower_http::services::ServeDir::new("./dist/"))
        .route(
            "/users",
            routes::users::router().with_state(user_svc.clone()),
        )
        .with_http_logging();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
