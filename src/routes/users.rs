use axum::{
    body::Body, extract::State, response, routing::{get, MethodRouter}, Form, RequestExt
};
use tera::Tera;

use crate::services::users::UserService;
use crate::{models, AppError};

use tracing::Instrument;

async fn get_users<UserSvc: UserService>(
    State((usersvc, tera)): State<(UserSvc, Tera)>,
    _req: axum::extract::Request,
) -> response::Result<axum::response::Html<impl Into<Body>>> {
    let users = usersvc
        .get_users(0, 200)
        .in_current_span()
        .await
        .map_err(AppError::from)?;

    Ok(response::Html(
        tera.render(
            "users/get.html",
            &tera::Context::from_value(
                serde_json::to_value(serde_json::json!({"users": users}))
                    .map_err(AppError::from)?,
            )
            .map_err(AppError::from)?,
        )
        .map_err(AppError::from)?,
    ))
}

async fn create_user<UserSvc: UserService>(
    State((usersvc, tera)): State<(UserSvc, Tera)>,
    // State(tera): State<Tera>,
    // Form(payload): Form<models::user::CreateUser>,
    req: axum::extract::Request,
) -> response::Result<response::Html<impl Into<Body>>> {
    let Form(payload): Form<models::user::CreateUser> = req.extract().await?;

    if payload.email.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            response::Html("email invalid".to_owned()),
        )
            .into());
    }

    let user = usersvc
        .create_user(&payload)
        .await
        .map_err(AppError::from)?;

    Ok(response::Html(
        tera.render(
            "users/create.html",
            &tera::Context::from_value(serde_json::to_value(user).map_err(AppError::from)?)
                .map_err(AppError::from)?,
        )
        .map_err(AppError::from)?,
    ))
}

type UserRoutesState<T> = (T, Tera);

pub fn router<UserSvc: UserService>() -> MethodRouter<UserRoutesState<UserSvc>> {
    get(get_users::<UserSvc>).post(create_user::<UserSvc>)
}
