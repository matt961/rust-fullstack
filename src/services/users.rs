use std::error::Error;

use deadpool_postgres::{Object, Pool};
use serde::{Deserialize, Serialize};

use super::DbService;

pub trait UserService {
    fn get_users<T>(&self, offset: u64, limit: u64) -> T;
}

#[derive(Clone)]
pub struct UserServiceDb {
    db: Pool,
}

impl UserServiceDb {
    fn new(db: Pool) -> Self {
        Self { db }
    }
}

impl UserServiceDb {
    pub async fn get_users(
        &self,
        offset: i32,
        limit: i64,
    ) -> Result<impl Iterator<Item = User>, deadpool_postgres::PoolError> {
        let conn = self.db.get().await?;
        let users = conn
            .query(
                "select * from users where id >= $1 limit $2;",
                &[&offset, &limit],
            )
            .await?;
        Ok(users.into_iter().map(|r| User {
            id: r.get::<_, i32>("id").into(),
            email: r.get::<_, String>("email"),
        }))
    }
}

impl DbService for UserServiceDb {
    fn new(db: Pool) -> Self {
        Self { db }
    }
}

// the input to our `create_user` handler
#[derive(Deserialize)]
pub struct CreateUser {
    pub email: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
pub struct User {
    pub id: i64,
    pub email: String,
}
