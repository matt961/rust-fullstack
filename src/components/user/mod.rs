use maud::{html, Markup};

use crate::models::user::User;

impl maud::Render for User {
    fn render(&self) -> Markup {
        html! {
            div class="user font-mono text-violet-900 text-2xl" {
                "User: " ( self.email )
            }
        }
    }
}
