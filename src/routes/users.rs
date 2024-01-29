use axum::{
    extract::State,
    response,
    routing::{get, MethodRouter},
    Form,
};
use maud::html;

use crate::services::users::UserService;
use crate::{components, models, AppError};

async fn get_users<S: UserService>(State(usersvc): State<S>) -> response::Result<maud::Markup> {
    let users = usersvc.get_users(0, 200).await.map_err(AppError::from);
    let users = users.map_err(|_| html! { p { "No users" } })?;

    Ok(components::user_list_component::render(&users))
}

async fn create_user<S: UserService>(
    State(usersvc): State<S>,
    Form(payload): Form<models::user::CreateUser>,
) -> response::Result<maud::Markup> {
    let user = usersvc.create_user(&payload).await;
    let user = user.map_err(|_| {
        return html! {
            p { "Error" }
        };
    })?;

    let r = html! {
        div hidden hx-get="/users"
            hx-trigger="load"
            hx-target="#user-list-component"
            hx-swap="innerHTML" {}
        p { "User " (user.email) " has been created. ID = " (user.id) }
    };
    Ok(r)
}

pub fn router<UserSvc: UserService>() -> MethodRouter<UserSvc> {
    get(get_users::<UserSvc>).post(create_user::<UserSvc>)
}
