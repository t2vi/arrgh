use std::net::SocketAddr;

/// Runtime config. Env var names match the .NET server + `docker/entrypoint.sh`
/// so the deployment contract is unchanged (ADR 0033).
#[derive(Debug, Clone)]
pub struct Config {
    /// Where the Rust server listens. Strangler-fig: .NET keeps :3000,
    /// Rust takes :3001, nginx routes per-prefix. Override with `RUST_BIND`.
    pub bind: SocketAddr,
    /// SQLite file path (`DatabasePath`). Unused until S2 (#124).
    pub database_path: String,
    /// Node plugin host base URL (`PluginHostUrl`).
    pub plugin_host_url: String,
    /// Download target dir (`DownloadDir`).
    pub download_dir: String,
    /// JWT signing secret (`JwtSecret`). Required from S2 on; optional now.
    pub jwt_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind = std::env::var("RUST_BIND")
            .unwrap_or_else(|_| "127.0.0.1:3001".into())
            .parse()?;

        let jwt_secret = env_opt("JwtSecret").or_else(|| env_opt("JWT_SECRET"));
        if jwt_secret.is_none() {
            tracing::warn!("JwtSecret not set — fine for S0, required once auth (S2) lands");
        }

        Ok(Self {
            bind,
            database_path: env_or("DatabasePath", "arrgh.db"),
            plugin_host_url: env_or("PluginHostUrl", "http://localhost:4000"),
            download_dir: env_or("DownloadDir", "./downloads"),
            jwt_secret,
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}
