mod config;
mod helpers;
mod middleware;
pub mod schema;
mod services;

use std::error::Error;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use config::tracing::HttpTracingExt;

use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;

use figment::{providers::Format, Figment};

use middleware::logging::HttpLoggingExt;
use services::users::{UserService, UserServiceDb};
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

    // create a new connection pool with the default config
    let config =
        AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(&cfg.database_url);
    let pool = Pool::builder(config).build()?;

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user).get(users))
        .with_http_logging()
        .with_state(UserServiceDb::new(pool.clone()));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

// basic handler that responds with a static string
async fn root() -> axum::response::Result<maud::Markup> {
    Ok(maud::html! {
        (maud::DOCTYPE)
            html {
                head {
                    script
                        src="https://unpkg.com/htmx.org@1.9.6"
                        integrity="sha384-FhXw7b6AlE/jyjlZH5iHa/tTe9EpJ1Y55RjcgPbjeWMskSxZt1v9qkxLJWNJaGni"
                        crossorigin="anonymous" {}
                }

                .user-component #"user-component" {
                    (helpers::WithAttr::<char>(
                            &[r#"hx-get="/users""#,
                            r#"hx-trigger="every 10s""#,
                            r#"hx-swap="innerHTML""#],
                            "div", None))
                }
            }
    })
}

async fn users(State(usersvc): State<UserServiceDb>) -> axum::response::Result<maud::Markup> {
    let users = usersvc.get_users(0, 200).await?;
    let markup = maud::html! {
        @for user in users {
            .test {
                ({
                    helpers::WithAttr(
                        &["hx-target-500=\"/yeeyee\""],
                        "article",
                        maud::html! {
                            .user {
                                pre {
                                    "User's email is " ( user.email )
                                }
                            }
                        }.into()
                    )
                })
            }
        }
    };
    Ok(markup)
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
