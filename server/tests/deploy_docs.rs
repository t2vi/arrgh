//! Every env var the deploy docs tell users to set for the `arrgh` container
//! must be one it actually reads — via `docker/entrypoint.sh`'s friendly-name
//! mapping or `src/config.rs` directly. `PLUGIN_INDEX_URL`, `PLUGIN_HOST_URL`
//! and `INDEX_INTERVAL_HOURS` were documented for months and did nothing.

use std::path::Path;

fn read(rel: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel))
        .unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// Backticked names in the first column of markdown table rows.
fn table_vars(md: &str) -> Vec<String> {
    md.lines()
        .filter_map(|l| l.strip_prefix("| `"))
        .filter_map(|l| l.split('`').next())
        .filter(|v| v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .map(String::from)
        .collect()
}

fn is_read(var: &str, entrypoint: &str, server_src: &str) -> bool {
    [format!("${var}"), format!("${{{var}")]
        .iter()
        .any(|p| entrypoint.contains(p.as_str()))
        || server_src.contains(&format!("\"{var}\""))
}

// spec: 010/FR-008
#[test]
fn documented_arrgh_env_vars_are_read_by_the_container() {
    let entrypoint = read("docker/entrypoint.sh");
    let server_src = [
        "server/src/config.rs",
        "server/src/main.rs",
        "server/src/lib.rs",
    ]
    .map(read)
    .join("\n");
    let compose = read("docs/deploy/docker-compose.md");
    // Only the `arrgh` service's table; plugin-host/cloakbrowser vars are theirs.
    let arrgh = compose
        .split("### `arrgh`")
        .nth(1)
        .and_then(|s| s.split("### ").next())
        .expect("docker-compose.md has an `arrgh` section");
    let configmap: Vec<String> = read("k8s/configmap.yaml")
        .split("\ndata:\n")
        .nth(1)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            l.strip_prefix("  ")?
                .split_once(':')
                .map(|(k, _)| k.to_string())
        })
        .filter(|k| !k.starts_with('#'))
        .collect();

    let documented = [
        table_vars(arrgh),
        table_vars(&read("docs/deploy/portainer.md")),
        configmap,
    ];
    let dead: Vec<&String> = documented
        .iter()
        .flatten()
        .filter(|v| !is_read(v, &entrypoint, &server_src))
        .collect();
    assert!(
        documented[0].len() >= 4,
        "parsed too few vars: {:?}",
        documented[0]
    );
    assert!(dead.is_empty(), "documented but never read: {dead:?}");
}
