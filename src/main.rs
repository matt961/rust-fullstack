mod background;
mod config;
mod error;
mod middleware;
mod models;
mod routes;
mod schema;
mod services;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::http::header;
use axum::Router;

use config::Env;
use diesel::Connection;
use diesel_async::pooled_connection::deadpool as diesel_deadpool;
use diesel_async::pooled_connection::deadpool::Hook;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;

use figment::{providers::Format, Figment};

use error::AppError;
use notify::Watcher;
use services::users::UserServiceDb;
use tera::Tera;
use tokio::spawn;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::*;
use tracing_forest::ForestLayer;
use tracing_subscriber::{prelude::*, EnvFilter};

use crate::background::posts_broker::PostsBroker;
use crate::middleware::logging::HttpLoggingExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg: config::AppCfg = Figment::new()
        .merge(figment::providers::Json::file("appsettings.json"))
        .merge(figment::providers::Env::prefixed("APP_"))
        .extract()?;

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(ForestLayer::default())
        .init();

    info!("application starting with environment: {:?}", cfg.env);

    // create a new connection pool with the default config
    let mgr =
        AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(&cfg.database_url);

    info!("Starting DB pool");
    let pgpool = diesel_deadpool::Pool::builder(mgr)
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
    let mig_res =
        <diesel::PgConnection as diesel_migrations::MigrationHarness<_>>::run_pending_migrations(
            &mut conn, MIGRATIONS,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
    for mig in mig_res {
        info!("Migration applied: {:?}", mig);
    }

    let user_svc = UserServiceDb::new(pgpool.clone());

    let tera: Arc<RwLock<_>> = Arc::new(Tera::new("src/templates/**/*")?.into());

    let mut tera_watcher = None;
    if cfg.env == Env::Development {
        info!("Development mode: setting up tera watcher");

        struct TeraTemplateWatcher {
            pub tera: Arc<RwLock<Tera>>,
            tokio_spawn_handle: tokio::runtime::Handle,
        }
        impl notify::EventHandler for TeraTemplateWatcher {
            fn handle_event(&mut self, event: notify::Result<notify::Event>) {
                match event {
                    Ok(e)
                        if e.kind
                            == notify::EventKind::Modify(notify::event::ModifyKind::Metadata(
                                notify::event::MetadataKind::Any,
                            ))
                            || e.kind
                                == notify::EventKind::Remove(notify::event::RemoveKind::File) =>
                    {
                        info!(?e, "tera file touched");
                        let s = self.tera.clone();
                        self.tokio_spawn_handle.spawn(async move {
                            let _ = s.write().await.full_reload();
                        });
                    }
                    Err(e) => error!(%e, "issue with watching tera templates"),
                    _ => {}
                }
            }
        }
        let tera_clone_watch = tera.clone();
        tera_watcher.replace(notify::recommended_watcher(TeraTemplateWatcher {
            tera: tera_clone_watch,
            tokio_spawn_handle: tokio::runtime::Handle::current(),
        })?);
    }
    if let Some(w) = tera_watcher.as_mut() {
        let _ = w
            .watch(
                Path::new("src/templates/"),
                notify::RecursiveMode::Recursive,
            )
            .inspect_err(|e| error!(%e, "issue with thing"));
    }

    let lapin_mgr = deadpool_lapin::Manager::new(
        &cfg.rabbitmq_url,
        lapin::ConnectionProperties::default()
            .with_executor(tokio_executor_trait::Tokio::current())
            .with_reactor(tokio_reactor_trait::Tokio),
    );
    let lapin_pool = deadpool_lapin::Pool::builder(lapin_mgr)
        .runtime(deadpool::Runtime::Tokio1)
        .create_timeout(Some(Duration::from_secs(5)))
        .max_size(100)
        .build()?;

    let posts_subscriber_mgr = Arc::new(background::posts_broker::PostsSubscriptionManager::new());
    let posts_broker = PostsBroker::new(posts_subscriber_mgr.clone(), lapin_pool.clone());

    // start posts broker background
    let _posts_broker_jhandle = spawn(
        posts_broker
            .instrument(info_span!("posts_broker_run"))
            .run(), // .instrument(info_span!("posts_broker_run")),
    );

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
            routes::users::router().with_state((user_svc.clone(), tera.clone())),
        )
        .nest(
            "/posts",
            routes::posts::router().with_state((
                tera.clone(),
                posts_subscriber_mgr.clone(),
                lapin_pool.clone(),
                pgpool.clone(),
            )),
        )
        .with_http_logging();

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("starting listening at {}", addr);
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
