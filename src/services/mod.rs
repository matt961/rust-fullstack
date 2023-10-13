pub mod users;

use deadpool_postgres as dpp;

pub trait DbService {
    fn new(db: dpp::Pool) -> Self;
}
