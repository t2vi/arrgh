use std::net::SocketAddr;

pub const DEFAULT_PLUGIN_INDEX_URL: &str =
    "https://raw.githubusercontent.com/t2vi/arrgh/main/plugin-index/index.json";
pub const DEFAULT_PLUGIN_INDEX_FALLBACK_URL: &str = "file:///app/plugin-index.json";

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
    /// Live plugin catalog (`PluginIndexUrl`) — `file://` or `http(s)://`;
    /// defaults to `main`'s raw `plugin-index/index.json` (spec 031, ADR 0035).
    pub plugin_index_url: String,
    /// Catalog copy baked into the image, used when the live one can't be
    /// read (`PluginIndexFallbackUrl`).
    pub plugin_index_fallback_url: String,
    /// Seed the bundled `external_sources` on first boot unless explicitly
    /// disabled (`SeedDefaultSources=false`) — tests use this to start from
    /// an empty table. Port of `Program.cs`'s inline seed block.
    pub seed_default_sources: bool,
    /// Metadata authority base URLs (ADR 0031, S6 #128). Real hosts by
    /// default; overridable so tests can point them at one local mock
    /// server — `reqwest::Client` can't be intercepted by host like .NET's
    /// fake `HttpMessageHandler` was, so the base URL itself is the seam.
    pub mangaupdates_url: String,
    pub anilist_url: String,
    pub mangadex_meta_url: String,
    pub wuxiaworld_meta_url: String,
    /// GitHub releases API base (`update_checker`'s poller, spec 007/FR-013). Same
    /// test seam as the metadata authorities above — only tests override it.
    pub github_releases_url: String,
    /// Download target dir (`DownloadDir`).
    pub download_dir: String,
    /// JWT signing secret (`JwtSecret`, else `JWT_SECRET`). `from_env` refuses to start without
    /// one (spec 003/FR-018); still an `Option` so tests can build a `Config` without auth.
    pub jwt_secret: Option<String>,
    /// Per-source Discover search bound. Deliberately not read from env —
    /// always `DISCOVER_SOURCE_TIMEOUT`; only tests override it (spec 021).
    pub discover_source_timeout: std::time::Duration,
    /// Whether a successful restore calls `std::process::exit` (spec 035). Always `true` in
    /// production — `docker-compose.yml`'s `restart: unless-stopped` brings the process back up
    /// against the just-restored file. Deliberately not read from env, same as
    /// `discover_source_timeout` above; only tests set this `false`, since a real exit would
    /// kill the whole test binary rather than just the one test.
    pub restart_on_restore: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// `from_env` over an injectable lookup — the unit-test seam for env parsing.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let env_or = |key: &str, default: &str| get(key).unwrap_or_else(|| default.to_string());
        let env_opt = |key: &str| get(key).filter(|v| !v.is_empty());

        // spec 016/FR-006: an unparseable RUST_BIND stops startup, never falls back.
        let bind = env_or("RUST_BIND", "127.0.0.1:3001")
            .parse()
            .map_err(|e| anyhow::anyhow!("RUST_BIND is not a valid host:port address: {e}"))?;

        // spec 003/FR-018: no secret → refuse to boot (Docker's entrypoint always sets one).
        let jwt_secret = env_opt("JwtSecret").or_else(|| env_opt("JWT_SECRET"));
        if jwt_secret.is_none() {
            anyhow::bail!("JwtSecret (or JWT_SECRET) is required — set it to any non-empty string");
        }

        Ok(Self {
            bind,
            database_path: env_or("DatabasePath", "arrgh.db"),
            plugin_host_url: env_or("PluginHostUrl", "http://localhost:4000"),
            plugin_index_url: env_or("PluginIndexUrl", DEFAULT_PLUGIN_INDEX_URL),
            plugin_index_fallback_url: env_or(
                "PluginIndexFallbackUrl",
                DEFAULT_PLUGIN_INDEX_FALLBACK_URL,
            ),
            seed_default_sources: env_or("SeedDefaultSources", "true") != "false",
            download_dir: env_or("DownloadDir", "./downloads"),
            mangaupdates_url: env_or(
                "MANGAUPDATES_URL",
                crate::metadata::mangaupdates::DEFAULT_BASE,
            ),
            anilist_url: env_or("ANILIST_URL", crate::metadata::anilist::DEFAULT_ENDPOINT),
            mangadex_meta_url: env_or("MANGADEX_META_URL", crate::metadata::mangadex::DEFAULT_BASE),
            wuxiaworld_meta_url: env_or("WUXIAWORLD_META_URL", "https://www.wuxiaworld.com"),
            github_releases_url: env_or(
                "GITHUB_RELEASES_URL",
                "https://api.github.com/repos/t2vi/arrgh/releases/latest",
            ),
            jwt_secret,
            discover_source_timeout: crate::api::discover::DISCOVER_SOURCE_TIMEOUT,
            restart_on_restore: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cfg(vars: &[(&str, &str)]) -> anyhow::Result<Config> {
        let env: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|k| env.get(k).cloned())
    }

    // spec: 003/FR-018
    #[test]
    fn missing_jwt_secret_refuses_to_start() {
        let err = cfg(&[]).unwrap_err().to_string();
        assert!(err.contains("JwtSecret"), "{err}");
        assert!(cfg(&[("JwtSecret", "")]).is_err(), "empty counts as unset");
    }

    // spec: 003/FR-018
    #[test]
    fn jwt_secret_precedence() {
        let c = cfg(&[("JWT_SECRET", "fallback")]).unwrap();
        assert_eq!(c.jwt_secret.as_deref(), Some("fallback"));
        let c = cfg(&[("JwtSecret", "primary"), ("JWT_SECRET", "fallback")]).unwrap();
        assert_eq!(c.jwt_secret.as_deref(), Some("primary"));
    }

    // spec: 016/FR-006
    #[test]
    fn rust_bind_default_override_and_invalid() {
        assert_eq!(
            cfg(&[("JwtSecret", "s")]).unwrap().bind.to_string(),
            "127.0.0.1:3001"
        );
        let c = cfg(&[("JwtSecret", "s"), ("RUST_BIND", "0.0.0.0:9000")]).unwrap();
        assert_eq!(c.bind.to_string(), "0.0.0.0:9000");
        assert!(cfg(&[("JwtSecret", "s"), ("RUST_BIND", "not-an-addr")]).is_err());
    }

    // spec: 007/FR-016
    #[test]
    fn seed_default_sources_only_off_for_exact_false() {
        let seed = |v: Option<&str>| {
            let mut vars = vec![("JwtSecret", "s")];
            if let Some(v) = v {
                vars.push(("SeedDefaultSources", v));
            }
            cfg(&vars).unwrap().seed_default_sources
        };
        assert!(seed(None));
        assert!(seed(Some("true")));
        assert!(seed(Some("no")));
        assert!(!seed(Some("false")));
    }
}
