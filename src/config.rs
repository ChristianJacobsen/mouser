use anyhow::{Context as _, Result};
use figment::{Figment, providers::Env};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(rename = "mouser_port", default = "default_port")]
    pub port: u16,
    #[serde(rename = "mouser_state_file", default = "default_state_file")]
    pub state_file: String,
    #[serde(rename = "mouser_mam_id")]
    pub mam_id: Option<String>,
    #[serde(rename = "mouser_check_interval", default = "default_check_interval")]
    pub check_interval: u64,
    #[serde(rename = "mouser_stale_threshold", default = "default_stale_threshold")]
    pub stale_threshold: u64,
    #[serde(rename = "mouser_user_agent", default = "default_user_agent")]
    pub user_agent: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        Figment::new()
            .merge(Env::raw())
            .extract()
            .context("failed to load configuration")
    }
}

fn default_port() -> u16 {
    7878
}

fn default_state_file() -> String {
    "/data/mouser/state.json".to_string()
}

fn default_check_interval() -> u64 {
    300
}

fn default_stale_threshold() -> u64 {
    86400
}

fn default_user_agent() -> String {
    format!("mouser/{}", env!("CARGO_PKG_VERSION"))
}
