mod config;
mod helpers;
mod middleware;
mod services;

use std::{error::Error, time::Duration};

use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use config::tracing::HttpTracingExt;
use deadpool_postgres as dpp;
use figment::{providers::Format, Figment};
use helpers::MapErr500;
use middleware::logging::HttpLoggingExt;
use services::{
    users::{UserService, UserServiceDb},
    DbService,
};
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cfg: config::DbCfg = Figment::new()
        .merge(figment::providers::Json::file("appsettings.json"))
        .merge(figment::providers::Env::prefixed("APP_"))
        .extract()?;

    // initialize tracing
    let fmtlayer = tracing_subscriber::fmt::layer();

    tracing_subscriber::registry()
        .with_http_tracing()
        .with(fmtlayer)
        .init();

    //deadpool
    let mut pg_conf = dpp::tokio_postgres::Config::new();
    pg_conf
        .host(&cfg.db_host)
        .dbname(&cfg.db_name)
        .user(&cfg.db_user)
        .password(&cfg.db_password)
        .port(15432)
        .connect_timeout(Duration::from_secs(5));

    let mgrcfg = dpp::ManagerConfig::default();
    let mgr = dpp::Manager::from_config(pg_conf, dpp::tokio_postgres::NoTls, mgrcfg);
    let pool = dpp::Pool::builder(mgr).max_size(8).build()?;

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user))
        .with_http_logging()
        .with_state(UserServiceDb::new(pool.clone()));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

// basic handler that responds with a static string
async fn root(
    State(usersvc): State<UserServiceDb>,
) -> axum::response::Result<axum::response::Html<Bytes>> {
    let users = usersvc.get_users(0, 200).await.map_err_500()?;
    let mut s = String::new();
    s.push_str(
        r#"
    <html>
        <body>
        "#,
    );
    for user in users {
        s.push_str(&format!("<pre>{}</pre>\n", &user.email));
    }
    s.push_str(
        r#"
        <body>
    <html>
        "#,
    );
    Ok(axum::response::Html(s.into()))
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<services::users::CreateUser>,
) -> impl IntoResponse {
    // insert your application logic here
    let user = services::users::User {
        id: 1337,
        email: payload.email,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}
