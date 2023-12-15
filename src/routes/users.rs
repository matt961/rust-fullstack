use axum::{
    extract::State,
    routing::{get, MethodRouter},
    Form,
};
use maud::html;

use crate::models;
use crate::services::users::UserService;

async fn users<S: UserService>(State(usersvc): State<S>) -> axum::response::Html<String> {
    let users = usersvc
        .get_users(0, 200)
        .await
        .map_err(crate::AppError::from);

    let r = match users {
        Ok(users) => crate::components::user_list_component::render(&users),
        Err(_) => html! { p { "No users" } },
    };
    axum::response::Html::from(r.into_string())
}

async fn create_user<S: UserService>(
    State(usersvc): State<S>,
    Form(payload): Form<models::user::CreateUser>,
) -> axum::response::Html<String> {
    let user = usersvc.create_user(&payload).await;

    let r = match user {
        Ok(user) => html! {
            div hidden hx-get="/users"
                        hx-trigger="load"
                        hx-target="#user-list-component"
                        hx-swap="innerHTML" {}
            p { "User " (user.email) " has been created. ID = " (user.id) }
            p { "User " (user.email) " has been created. ID = " (user.id) }
        },
        Err(_) => html! {
            p { "Error" }
        },
    };

    axum::response::Html::from(r.into_string())
}

pub fn router<UserSvc: UserService>() -> MethodRouter<UserSvc> {
    get(users::<UserSvc>).post(create_user::<UserSvc>)
}
