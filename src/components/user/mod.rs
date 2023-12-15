use maud::{html, Markup};

use crate::models::user::User;

impl maud::Render for User {
    fn render(&self) -> Markup {
        html! {
            .user {
                "User: " ( self.email )
            }
        }
    }
}
