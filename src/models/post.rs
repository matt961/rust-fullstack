use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Insertable)]
#[diesel(table_name = crate::schema::posts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreatePost {
}

#[derive(Serialize, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::posts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Post {
}
