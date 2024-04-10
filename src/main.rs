mod config;
mod error;
mod middleware;
mod models;
mod routes;
mod schema;
mod services;

use axum::http::header;
use axum::Router;

use diesel::Connection;
use diesel_async::pooled_connection::deadpool::{Hook, Pool};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;

use diesel_migrations::MigrationHarness;
use figment::{providers::Format, Figment};

use error::AppError;
use services::users::UserServiceDb;
use tera::Tera;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::*;
use tracing_forest::ForestLayer;
use tracing_subscriber::{prelude::*, EnvFilter};

use crate::middleware::logging::HttpLoggingExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg: config::DbCfg = Figment::new()
        .merge(figment::providers::Json::file("appsettings.json"))
        .merge(figment::providers::Env::prefixed("APP_"))
        .extract()?;

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(ForestLayer::default())
        .init();

    // create a new connection pool with the default config
    let mgr =
        AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(&cfg.database_url);

    info!("Starting DB pool");
    let pool = Pool::builder(mgr)
        .max_size(10)
        .pre_recycle(Hook::async_fn(|conn, metrics| {
            tracing::trace_span!("dbpool::pre_recycle").in_scope(|| {
                let c = std::ptr::addr_of!(conn);
                tracing::trace!(?c, ?metrics, "Pre-recycle");
                Box::pin(std::future::ready(Ok(())))
            })
        }))
        .post_create(Hook::async_fn(|conn, metrics| {
            tracing::trace_span!("dbpool::post_create").in_scope(|| {
                let c = std::ptr::addr_of!(conn);
                tracing::trace!(?c, ?metrics, "Post-create");
                Box::pin(std::future::ready(Ok(())))
            })
        }))
        .runtime(deadpool::Runtime::Tokio1)
        .build()?;

    const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
        diesel_migrations::embed_migrations!("migrations/");
    let mut conn = diesel::PgConnection::establish(&cfg.database_url)?;
    let mig_res = <diesel::PgConnection as MigrationHarness<_>>::run_pending_migrations(
        &mut conn, MIGRATIONS,
    )
    .map_err(|e| anyhow::anyhow!(e))?;
    for mig in mig_res {
        info!("Migration applied: {:?}", mig);
    }

    let user_svc = UserServiceDb::new(pool.clone());

    let tera = Tera::new("src/templates/**/*")?;

    let app = Router::new()
        .nest_service(
            "/",
            ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::if_not_present(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("max-age=13420"),
                ))
                .layer(CompressionLayer::new())
                .service(tower_http::services::ServeDir::new("./dist/")),
        )
        .route(
            "/users",
            routes::users::router()
            .with_state((user_svc.clone(), tera.clone())),
        )
        .with_http_logging();

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("started listening on {}", addr);
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
