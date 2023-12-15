use diesel::prelude::*;
use serde::{Deserialize, Serialize};

// the input to our `create_user` handler
#[derive(Deserialize, Insertable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreateUser {
    pub email: String,
}

// the output to our `create_user` handler
#[derive(Serialize, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: i32,
    pub email: String,
}
