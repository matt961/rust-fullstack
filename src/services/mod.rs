use diesel_async::AsyncPgConnection;

pub mod users;

pub type Pool = diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>;

pub trait Svc: Clone + Send + Sync + 'static {}
