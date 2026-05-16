use anyhow::Result;
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub manga_dir: String,
    pub index_interval_hours: u64,
    pub bind_addr: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://arrgh.db".to_string()),
            manga_dir: env::var("MANGA_DIR")
                .unwrap_or_else(|_| "./manga".to_string()),
            index_interval_hours: env::var("INDEX_INTERVAL_HOURS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6),
            bind_addr: env::var("BIND_ADDR").ok(),
        })
    }
}
