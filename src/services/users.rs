use axum::async_trait;
use diesel::prelude::*;

use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};

use crate::helpers::error::AppError;

use super::Pool;

#[async_trait]
pub trait UserService<E> {
    async fn get_users(&self, offset: i32, limit: i64)
        -> Result<Box<dyn Iterator<Item = User>>, E>;
}

#[derive(Clone)]
pub struct UserServiceDb {
    db: Pool,
}

#[async_trait]
impl UserService<AppError> for UserServiceDb {
    async fn get_users(
        &self,
        offset: i32,
        limit: i64,
    ) -> Result<Box<dyn Iterator<Item = User>>, AppError> {
        use crate::schema::users::dsl::*;

        let mut conn = self.db.get().await?;
        let us: Vec<User> = users
            .filter(id.gt(offset))
            .limit(limit)
            .select(User::as_select())
            .load(&mut conn)
            .await?;
        Ok(Box::new(us.into_iter()))
    }
}

impl UserServiceDb {
    pub fn new(db: Pool) -> Self {
        Self { db }
    }
}

// the input to our `create_user` handler
#[derive(Deserialize)]
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
