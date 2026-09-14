//! `/api/discover` domain logic (ADR 0033 S6 #128; ADR 0031 fan-out/dedup;
//! ADR 0015/0021 designated authorities). Port of `Api/Discover.cs`.
//!
//! Two things .NET keeps but never actually calls are **not** ported here,
//! same "don't port dead code" call as `metadata::mod`'s E-Hentai skip:
//! `SearchCandidates`/`KnownNorms` (defined, unit-tested, never invoked from
//! `MatchSourcesAsync` or anywhere else — that function builds its
//! normalized target/aliases inline instead).

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{chapters, sources, titles};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DiscoverResult {
    pub mangaupdates_id: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub author: Option<String>,
    pub year: Option<i32>,
    pub tags: Option<String>,
    pub content_type: String,
    #[serde(default)]
    pub in_library: bool,
    pub library_id: Option<String>,
    pub source: String,
    #[serde(default)]
    pub is_explicit: bool,
}

// ── pure helpers ─────────────────────────────────────────────────────────

pub fn normalize_title(title: &str) -> String {
    let normalized: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub const AUTHORITY_ORDER: [&str; 6] = [
    "mangaupdates",
    "anilist",
    "mangadex",
    "novelupdates",
    "wuxiaworld",
    "nhentai",
];

pub fn designated_authority(content_type: &str) -> &'static str {
    match content_type.to_lowercase().as_str() {
        "manhwa" => "anilist",
        "manhua" => "mangadex",
        "novel" | "web novel" | "light novel" => "novelupdates",
        "hentai" => "nhentai",
        _ => "mangaupdates",
    }
}

/// ADR 0031: MangaUpdates is manga-authority only — strip non-manga/one-shot
/// results before dedup so MU novel/manhwa/manhua results can't survive when
/// the designated authority returns nothing.
pub fn filter_mu_scope(results: Vec<DiscoverResult>) -> Vec<DiscoverResult> {
    results
        .into_iter()
        .filter(|r| matches!(r.content_type.to_lowercase().as_str(), "manga" | "one-shot"))
        .collect()
}

pub fn deduplicate(results: Vec<DiscoverResult>) -> Vec<DiscoverResult> {
    let mut groups: Vec<(String, String, Vec<DiscoverResult>)> = Vec::new();
    for r in results {
        let key_title = normalize_title(&r.title);
        let key_ct = r.content_type.to_lowercase();
        if let Some(g) = groups
            .iter_mut()
            .find(|(t, c, _)| *t == key_title && *c == key_ct)
        {
            g.2.push(r);
        } else {
            groups.push((key_title, key_ct, vec![r]));
        }
    }

    groups
        .into_iter()
        .map(|(_, content_type, mut items)| {
            if items.len() == 1 {
                return items.pop().unwrap();
            }
            let designated = designated_authority(&content_type);
            let idx = items
                .iter()
                .position(|r| r.source == designated)
                .unwrap_or(0);
            items.into_iter().nth(idx).unwrap()
        })
        .collect()
}

/// Merge fan-out results: nhentai-upgrade pass, dedup, then sort by
/// `AUTHORITY_ORDER`.
pub fn merge_fan_out(results: Vec<DiscoverResult>) -> Vec<DiscoverResult> {
    let nhentai_norms: std::collections::HashSet<String> = results
        .iter()
        .filter(|r| r.source == "nhentai")
        .map(|r| normalize_title(&r.title))
        .collect();

    let pre_dedup = if nhentai_norms.is_empty() {
        results
    } else {
        let mut consumed = std::collections::HashSet::new();
        let mut results = results;
        for r in &mut results {
            if r.source == "nhentai" || !r.is_explicit || r.content_type != "manga" {
                continue;
            }
            let norm = normalize_title(&r.title);
            let matched = nhentai_norms
                .iter()
                .find(|nh| norm == **nh || norm.starts_with(&format!("{nh} ")));
            let Some(matched) = matched.cloned() else {
                continue;
            };
            r.content_type = "hentai".to_string();
            r.is_explicit = true;
            consumed.insert(matched);
        }
        results
            .into_iter()
            .filter(|r| !(r.source == "nhentai" && consumed.contains(&normalize_title(&r.title))))
            .collect()
    };

    let mut deduped = deduplicate(pre_dedup);
    deduped.sort_by_key(|r| {
        AUTHORITY_ORDER
            .iter()
            .position(|a| *a == r.source)
            .unwrap_or(usize::MAX)
    });
    deduped
}

pub fn title_matches(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let max_len = a.chars().count().max(b.chars().count());
    if max_len == 0 {
        return true;
    }
    levenshtein(a, b) * 5 <= max_len
}

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut row: Vec<usize> = (0..=n).collect();
    for i in 1..=m {
        let mut prev = row[0];
        row[0] = i;
        for j in 1..=n {
            let old = row[j];
            row[j] = if a[i - 1] == b[j - 1] {
                prev
            } else {
                1 + prev.min(row[j]).min(row[j - 1])
            };
            prev = old;
        }
    }
    row[n]
}

/// Strips a trailing parenthesised qualifier like `"Naruto (Manga)"` →
/// `"Naruto"`, used to clean up user-typed search-result titles before
/// storing them as the library title. `None` when there's nothing to strip.
pub fn strip_search_qualifier(s: &str) -> Option<String> {
    let s = s.trim();
    let open_idx = s.rfind('(')?;
    let suffix = &s[open_idx..];
    if !suffix.ends_with(')') || suffix.chars().count() < 3 || suffix.chars().count() > 20 {
        return None;
    }
    let stripped = s[..open_idx].trim_end();
    if stripped.is_empty() || stripped == s {
        None
    } else {
        Some(stripped.to_string())
    }
}

pub fn is_hentai_tag(tags: Option<&str>) -> bool {
    tags.map(|t| {
        t.split(',')
            .any(|t| t.trim().eq_ignore_ascii_case("hentai"))
    })
    .unwrap_or(false)
}

// ── in-library check ─────────────────────────────────────────────────────

pub async fn check_in_library(
    pool: &SqlitePool,
    user_id: &str,
    metadata_source: &str,
    mu_id: &str,
    title: &str,
    content_type: &str,
) -> sqlx::Result<(bool, Option<String>)> {
    if !metadata_source.is_empty() && !mu_id.is_empty() {
        if let Some(id) =
            titles::find_owned_id_by_metadata_source(pool, user_id, metadata_source, mu_id).await?
        {
            return Ok((true, Some(id)));
        }
    }
    if !mu_id.is_empty() {
        if let Some(id) = titles::find_owned_id_by_mangaupdates_id(pool, user_id, mu_id).await? {
            return Ok((true, Some(id)));
        }
    }
    let norm_title = normalize_title(title);
    let owned = titles::list_owned_by_content_type(pool, user_id, content_type).await?;
    if let Some((id, _)) = owned.iter().find(|(_, t)| normalize_title(t) == norm_title) {
        return Ok((true, Some(id.clone())));
    }
    Ok((false, None))
}

// ── cover cache (`title_meta`) ───────────────────────────────────────────

pub async fn enrich_cover(pool: &SqlitePool, result: &mut DiscoverResult) -> sqlx::Result<()> {
    let key = normalize_title(&result.title);
    let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT cover_local_path, cover_cdn_url FROM title_meta WHERE title_key = ?",
    )
    .bind(&key)
    .fetch_optional(pool)
    .await?;
    let Some((local_path, cdn_url)) = row else {
        return Ok(());
    };

    if local_path.is_some() {
        result.cover_url = Some(format!(
            "/api/media/meta-cover?key={}",
            urlencoding::encode(&key)
        ));
    } else if result.cover_url.is_none() {
        result.cover_url = cdn_url;
    }
    Ok(())
}

/// Seeds/refreshes the `title_meta` cover-cache row (best-effort — errors
/// swallowed, mirrors `SeedAndCacheCoverAsync`).
pub async fn seed_and_cache_cover(
    pool: &SqlitePool,
    http: &reqwest::Client,
    title: &str,
    cdn_url: &str,
    download_dir: &str,
    source: &str,
    source_id: &str,
) {
    let _: anyhow::Result<()> = async {
        let key = normalize_title(title);
        let now = ef_timestamp_now();
        sqlx::query(
            "INSERT INTO title_meta (title_key, cover_cdn_url, fetched_at, source, source_id, chapter_count) \
             VALUES (?, ?, ?, ?, ?, 0) \
             ON CONFLICT(title_key) DO UPDATE SET \
                 cover_cdn_url = COALESCE(title_meta.cover_cdn_url, excluded.cover_cdn_url), \
                 fetched_at    = excluded.fetched_at",
        )
        .bind(&key)
        .bind(cdn_url)
        .bind(&now)
        .bind(source)
        .bind(source_id)
        .execute(pool)
        .await?;

        let bytes = http
            .get(cdn_url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let ext = cdn_url.split('?').next().unwrap_or(cdn_url).rsplit('.').next().unwrap_or("jpg").to_lowercase();
        let safe_name: String = key.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
        let dir = std::path::Path::new(download_dir).join("_meta");
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join(format!("{safe_name}.{ext}"));
        tokio::fs::write(&path, &bytes).await?;

        sqlx::query("UPDATE title_meta SET cover_local_path = ? WHERE title_key = ?")
            .bind(path.to_string_lossy().as_ref())
            .bind(&key)
            .execute(pool)
            .await?;
        Ok(())
    }
    .await;
}

/// Downloads a title's cover to `{download_dir}/_covers/{title_id}.{ext}`
/// and updates `titles.cover_url` to the local path (best-effort, mirrors
/// `DownloadCoverAsync`).
pub async fn download_cover(
    pool: &SqlitePool,
    http: &reqwest::Client,
    title_id: &str,
    cdn_url: &str,
    download_dir: &str,
) {
    let _: anyhow::Result<()> = async {
        let bytes = http
            .get(cdn_url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let ext = cdn_url
            .split('?')
            .next()
            .unwrap_or(cdn_url)
            .rsplit('.')
            .next()
            .unwrap_or("jpg")
            .to_lowercase();
        let dir = std::path::Path::new(download_dir).join("_covers");
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join(format!("{title_id}.{ext}"));
        tokio::fs::write(&path, &bytes).await?;

        sqlx::query("UPDATE titles SET cover_url = ? WHERE id = ?")
            .bind(path.to_string_lossy().as_ref())
            .bind(title_id)
            .execute(pool)
            .await?;
        Ok(())
    }
    .await;
}

/// Resolves a `/api/media/meta-cover?key=...` URL to its cached CDN URL —
/// port of `AddManga`'s cover-resolution step.
pub async fn resolve_meta_cover(
    pool: &SqlitePool,
    cover_url: &str,
) -> sqlx::Result<Option<String>> {
    const PREFIX: &str = "/api/media/meta-cover?key=";
    let Some(encoded) = cover_url.strip_prefix(PREFIX) else {
        return Ok(Some(cover_url.to_string()));
    };
    let key = urlencoding::decode(encoded)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| encoded.to_string());
    sqlx::query_scalar("SELECT cover_cdn_url FROM title_meta WHERE title_key = ?")
        .bind(&key)
        .fetch_optional(pool)
        .await
}

// ── source matching (ADR 0013 + ADR 0016) ───────────────────────────────

#[derive(Deserialize)]
struct PluginSearchResult {
    id: Option<String>,
    title: Option<String>,
}

/// After a title is added: query `external_sources` by content_type, call
/// plugin-host `/search`, match by normalized title (or alias), create
/// `title_sources` + sync chapters. Port of `Discover.MatchSourcesAsync`.
/// Best-effort throughout — logs to `sync_log`/`sync_warnings`, never
/// propagates errors to the caller.
pub async fn match_sources(
    pool: &SqlitePool,
    http: &reqwest::Client,
    plugin_host_url: &str,
    title_id: &str,
    title_name: &str,
    content_type: &str,
    is_explicit: bool,
) {
    if title_name.trim().is_empty() || content_type.trim().is_empty() {
        return;
    }

    let include_hentai = content_type == "manga" && is_explicit;
    let Ok(candidate_sources) =
        sources::matching_for_content_type(pool, content_type, include_hentai).await
    else {
        return;
    };
    if candidate_sources.is_empty() {
        return;
    }

    let norm_target = normalize_title(title_name);
    let aliases = titles::list_title_aliases(pool, title_id)
        .await
        .unwrap_or_default();
    let norm_aliases: Vec<String> = aliases.iter().map(|a| normalize_title(a)).collect();

    for (source_key, _priority) in &candidate_sources {
        let search_url = format!(
            "{}/{}/search?q={}",
            plugin_host_url.trim_end_matches('/'),
            source_key,
            urlencoding::encode(title_name)
        );

        let resp = match http.get(&search_url).send().await {
            Ok(r) => r,
            Err(e) if e.is_timeout() => {
                // Soft failure — CF-protected source without CloakBrowser, or
                // plugin-host unreachable. Log but no sync_warning.
                titles::append_sync_log(pool, title_id, &format!("Source {source_key} timed out"))
                    .await;
                continue;
            }
            Err(e) if e.is_connect() => {
                titles::append_sync_log(
                    pool,
                    title_id,
                    &format!("Source {source_key} unreachable"),
                )
                .await;
                continue;
            }
            Err(e) => {
                titles::append_sync_log(pool, title_id, &format!("Error from {source_key}: {e}"))
                    .await;
                titles::append_sync_warning(pool, title_id, source_key, &e.to_string()).await;
                continue;
            }
        };

        if !resp.status().is_success() {
            titles::append_sync_log(
                pool,
                title_id,
                &format!(
                    "Source {source_key} unavailable ({})",
                    resp.status().as_u16()
                ),
            )
            .await;
            continue;
        }

        let results: Vec<PluginSearchResult> = match resp.json().await {
            Ok(r) => r,
            Err(e) => {
                titles::append_sync_log(pool, title_id, &format!("Error from {source_key}: {e}"))
                    .await;
                titles::append_sync_warning(pool, title_id, source_key, &e.to_string()).await;
                continue;
            }
        };
        if results.is_empty() {
            titles::append_sync_log(pool, title_id, &format!("No results from {source_key}")).await;
            continue;
        }

        let matched = results.iter().find(|r| {
            let norm_result = normalize_title(r.title.as_deref().unwrap_or(""));
            title_matches(&norm_result, &norm_target)
                || norm_aliases
                    .iter()
                    .any(|na| title_matches(&norm_result, na))
        });
        let Some(matched) = matched else {
            titles::append_sync_log(
                pool,
                title_id,
                &format!("No title match on {source_key} (searched \"{title_name}\")"),
            )
            .await;
            continue;
        };
        let Some(source_id) = matched.id.as_deref().filter(|s| !s.is_empty()) else {
            continue;
        };

        titles::append_sync_log(
            pool,
            title_id,
            &format!("Matched {source_key}:{source_id} — syncing chapters…"),
        )
        .await;

        if !titles::has_title_source_for(pool, title_id, source_key)
            .await
            .unwrap_or(false)
        {
            let _ = titles::insert_title_source(pool, title_id, source_key, source_id).await;
        }

        match chapters::sync_from_source(
            pool,
            http,
            plugin_host_url,
            title_id,
            content_type,
            source_key,
            source_id,
        )
        .await
        {
            Ok(count) => {
                titles::append_sync_log(
                    pool,
                    title_id,
                    &format!("Synced {count} chapter(s) from {source_key}"),
                )
                .await
            }
            Err(e) => {
                titles::append_sync_log(
                    pool,
                    title_id,
                    &format!("Error syncing from {source_key}: {e}"),
                )
                .await
            }
        }
    }

    if !titles::has_any_source_link(pool, title_id)
        .await
        .unwrap_or(true)
    {
        titles::append_sync_warning(
            pool,
            title_id,
            "source-matching",
            "No sources could be matched for this title. Check that the title name is correct and the plugins are reachable.",
        )
        .await;
    }
}

fn ef_timestamp_now() -> String {
    use time::macros::format_description;
    time::OffsetDateTime::now_utc()
        .format(&format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]"
        ))
        .expect("format is a static valid pattern")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(source: &str, title: &str, content_type: &str) -> DiscoverResult {
        DiscoverResult {
            title: title.to_string(),
            content_type: content_type.to_string(),
            source: source.to_string(),
            mangaupdates_id: "id".to_string(),
            status: "unknown".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn normalize_title_strips_punctuation_and_lowercases() {
        assert_eq!(normalize_title("Naruto: Shippuden!"), "naruto shippuden");
        assert_eq!(normalize_title("  Multiple   Spaces "), "multiple spaces");
    }

    #[test]
    fn designated_authority_per_content_type() {
        assert_eq!(designated_authority("manhwa"), "anilist");
        assert_eq!(designated_authority("manhua"), "mangadex");
        assert_eq!(designated_authority("novel"), "novelupdates");
        assert_eq!(designated_authority("hentai"), "nhentai");
        assert_eq!(designated_authority("manga"), "mangaupdates");
        assert_eq!(designated_authority("unknown"), "mangaupdates");
    }

    #[test]
    fn deduplicate_designated_authority_wins() {
        let results = vec![
            result("mangaupdates", "Solo Leveling", "manhwa"),
            result("anilist", "Solo Leveling", "manhwa"),
        ];
        let deduped = deduplicate(results);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].source, "anilist");
    }

    #[test]
    fn deduplicate_keeps_different_content_types_separate() {
        let results = vec![
            result("mangaupdates", "X", "manga"),
            result("novelupdates", "X", "novel"),
        ];
        assert_eq!(deduplicate(results).len(), 2);
    }

    #[test]
    fn merge_fan_out_orders_by_authority() {
        let results = vec![
            result("novelupdates", "A", "novel"),
            result("mangaupdates", "B", "manga"),
        ];
        let merged = merge_fan_out(results);
        assert_eq!(merged[0].source, "mangaupdates");
        assert_eq!(merged[1].source, "novelupdates");
    }

    #[test]
    fn merge_fan_out_upgrades_matching_explicit_manga_to_hentai() {
        let mut nhentai = result("nhentai", "Kayanetori", "hentai");
        nhentai.is_explicit = true;
        let mut mu = result(
            "mangaupdates",
            "Kayanetori Kaya-nee Series Aizou Ban",
            "manga",
        );
        mu.is_explicit = true;
        let merged = merge_fan_out(vec![nhentai, mu]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, "mangaupdates");
        assert_eq!(merged[0].content_type, "hentai");
        assert!(merged[0].is_explicit);
    }

    #[test]
    fn merge_fan_out_does_not_upgrade_non_explicit_result() {
        let nhentai = result("nhentai", "Berserk", "hentai");
        let mut mu = result("mangaupdates", "Berserk", "manga");
        mu.is_explicit = false;
        let merged = merge_fan_out(vec![nhentai, mu]);
        // no upgrade → both survive as separate content_types
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn merge_fan_out_upgrades_on_word_boundary_prefix() {
        // .NET's actual `MergeFanOut` matches exact OR word-boundary prefix
        // (`norm.StartsWith(nh + " ")`) — "Berserk dj Lightning" DOES upgrade
        // against nhentai "Berserk" under that rule, same as the source.
        let nhentai = result("nhentai", "Berserk", "hentai");
        let mut mu = result("mangaupdates", "Berserk dj Lightning", "manga");
        mu.is_explicit = true;
        let merged = merge_fan_out(vec![nhentai, mu]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].content_type, "hentai");
    }

    #[test]
    fn merge_fan_out_does_not_upgrade_on_substring_without_word_boundary() {
        // "Berserker" contains "Berserk" as a prefix but with no following
        // space — must not upgrade (word-boundary required).
        let nhentai = result("nhentai", "Berserk", "hentai");
        let mut mu = result("mangaupdates", "Berserker", "manga");
        mu.is_explicit = true;
        let merged = merge_fan_out(vec![nhentai, mu]);
        assert_eq!(merged.len(), 2);
        assert_eq!(
            merged
                .iter()
                .find(|r| r.source == "mangaupdates")
                .unwrap()
                .content_type,
            "manga"
        );
    }

    #[test]
    fn filter_mu_scope_keeps_only_manga_and_one_shot() {
        let results = vec![
            result("mangaupdates", "A", "manga"),
            result("mangaupdates", "B", "novel"),
            result("mangaupdates", "C", "one-shot"),
        ];
        assert_eq!(filter_mu_scope(results).len(), 2);
    }

    #[test]
    fn title_matches_exact_and_fuzzy() {
        assert!(title_matches("naruto", "naruto"));
        assert!(title_matches("naruto shippuden", "naruto shippudan")); // 1 char diff, len 15*5=75>=1
        assert!(!title_matches("naruto", "bleach"));
    }

    #[test]
    fn levenshtein_basic() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("same", "same"), 0);
    }

    #[test]
    fn strip_search_qualifier_strips_trailing_parens() {
        assert_eq!(
            strip_search_qualifier("Naruto (Manga)").as_deref(),
            Some("Naruto")
        );
        assert_eq!(strip_search_qualifier("Naruto"), None);
        assert_eq!(strip_search_qualifier("(Just Parens)"), None); // stripped would be empty
    }

    #[test]
    fn is_hentai_tag_detects_case_insensitive() {
        assert!(is_hentai_tag(Some("Action,Hentai")));
        assert!(!is_hentai_tag(Some("Action,Adult")));
        assert!(!is_hentai_tag(None));
    }
}
