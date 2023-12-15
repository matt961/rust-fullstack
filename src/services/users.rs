use axum::async_trait;
use diesel::prelude::*;

use crate::models::user::*;
use diesel_async::RunQueryDsl;

use crate::schema;

use super::Pool;

#[async_trait]
pub trait UserService<E = anyhow::Error>: Clone + Send + Sync + 'static {
    async fn get_users(&self, offset: i32, limit: i64) -> Result<Vec<User>, E>;
    async fn create_user(&self, user: &CreateUser) -> Result<User, E>;
}

#[derive(Clone)]
pub struct UserServiceDb {
    db: Pool,
}

#[async_trait]
impl UserService<anyhow::Error> for UserServiceDb {
    async fn get_users(&self, offset: i32, limit: i64) -> Result<Vec<User>, anyhow::Error> {
        use schema::users::dsl::*;

        let mut conn = self.db.get().await?;
        let us: Vec<User> = users
            .filter(id.gt(offset))
            .limit(limit)
            .select(User::as_select())
            .load(&mut conn)
            .await?;
        Ok(us)
    }

    async fn create_user(&self, u: &CreateUser) -> Result<User, anyhow::Error> {
        use schema::users::dsl::*;

        let mut conn = self.db.get().await?;

        let user = diesel::insert_into(users)
            .values(u)
            .get_result::<User>(&mut conn)
            .await?;

        Ok(user)
    }
}

impl UserServiceDb {
    pub fn new(db: Pool) -> Self {
        Self { db }
    }
}
