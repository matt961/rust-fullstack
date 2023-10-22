pub mod tracing;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct DbCfg {
    pub database_url: String,
}
