use crate::models::user::User;
use maud::{html, Markup};

pub fn render(us: &[User]) -> Markup {
    html! {
        @for user in us {
            ( user )
        }
    }
}
