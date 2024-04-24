use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Insertable)]
#[diesel(table_name = crate::schema::posts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreatePost {
    user_id: i32,
    post_content: String,
}

#[derive(Serialize, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::posts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Post {
    id: Uuid,
    user_id: i32,
    post_content: String,
    tags: Vec<Option<String>>,
}
