use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppCfg {
    pub database_url: String,
    pub rabbitmq_url: String,
    pub env: Env,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub enum Env {
    Production,
    Development,
}
