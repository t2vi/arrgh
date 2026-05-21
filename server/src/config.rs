use anyhow::Result;
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub download_dir: String,
    pub index_interval_hours: u64,
    pub bind_addr: Option<String>,
    pub plugin_host_url: String,
    pub plugin_index_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://arrgh.db".to_string()),
            download_dir: env::var("DOWNLOAD_DIR")
                .unwrap_or_else(|_| "./downloads".to_string()),
            index_interval_hours: env::var("INDEX_INTERVAL_HOURS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6),
            bind_addr: env::var("BIND_ADDR").ok(),
            plugin_host_url: env::var("PLUGIN_HOST_URL")
                .unwrap_or_else(|_| "http://plugin-host:4000".to_string()),
            plugin_index_url: env::var("PLUGIN_INDEX_URL")
                .unwrap_or_else(|_| "file:///app/plugin-index.json".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    // Env vars are process-global — serialize tests that mutate them.
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn defaults_when_env_absent() {
        let _g = env_lock();
        for k in ["DATABASE_URL", "DOWNLOAD_DIR", "INDEX_INTERVAL_HOURS", "BIND_ADDR", "PLUGIN_HOST_URL", "PLUGIN_INDEX_URL"] {
            env::remove_var(k);
        }
        let cfg = Config::from_env().unwrap();
        assert_eq!(cfg.database_url, "sqlite://arrgh.db");
        assert_eq!(cfg.download_dir, "./downloads");
        assert_eq!(cfg.index_interval_hours, 6);
        assert!(cfg.bind_addr.is_none());
        assert_eq!(cfg.plugin_host_url, "http://plugin-host:4000");
    }

    #[test]
    fn index_interval_unparseable_falls_back_to_default() {
        let _g = env_lock();
        env::set_var("INDEX_INTERVAL_HOURS", "not-a-number");
        let cfg = Config::from_env().unwrap();
        assert_eq!(cfg.index_interval_hours, 6);
        env::remove_var("INDEX_INTERVAL_HOURS");
    }

    #[test]
    fn bind_addr_present_when_set() {
        let _g = env_lock();
        env::set_var("BIND_ADDR", "0.0.0.0:8080");
        let cfg = Config::from_env().unwrap();
        assert_eq!(cfg.bind_addr, Some("0.0.0.0:8080".to_string()));
        env::remove_var("BIND_ADDR");
    }
}
