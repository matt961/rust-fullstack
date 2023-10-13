pub mod tracing;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct DbCfg {
    pub db_host: String,
    pub db_user: String,
    pub db_password: String,
    pub db_name: String,
}
