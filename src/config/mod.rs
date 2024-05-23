pub mod tracing;

use serde::Deserialize;

#[derive(Debug)]
#[derive(Deserialize)]
pub struct AppCfg {
    pub database_url: String,
    pub rabbitmq_url: String,
    pub env: Env
}

#[derive(Debug)]
#[derive(PartialEq, Eq)]
#[derive(Deserialize)]
pub enum Env {
    Production,
    Development
}
