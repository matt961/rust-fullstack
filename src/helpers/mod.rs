use axum::http::StatusCode;
use deadpool_postgres::PoolError;

pub trait MapErr500<T> {
    fn map_err_500(self) -> axum::response::Result<T>;
}

impl<T> MapErr500<T> for Result<T, PoolError> {
    fn map_err_500(self) -> axum::response::Result<T> {
        self.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into())
    }
}
