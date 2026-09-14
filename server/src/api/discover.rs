//! `/api/discover` — port of `Api/Discover.cs` (ADR 0033 S6 #128; ADR 0031
//! fan-out/dedup; ADR 0032 trending lanes). E-Hentai is dead code in .NET
//! (superseded by nhentai) and isn't ported — see `crate::metadata`'s module
//! doc.
//!
//! Unlike titles/progress/chapters, Discover doesn't share hot tables with
//! that S4-S7 block (it *creates* titles/chapters via `AddManga`, but reads
//! nothing back through routes still on .NET) — the ADR calls it out as its
//! own self-contained gate, so `/api/discover` flips in `docker/nginx.conf`
//! as soon as this phase is green, without waiting for S7.

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::api::titles::TitleDto;
use crate::auth::Claims;
use crate::discover::{self, DiscoverResult};
use crate::error::{AppError, AppResult};
use crate::metadata;
use crate::state::AppState;
use crate::titles;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(search))
        .route("/trending/manga", get(trending_manga))
        .route("/trending/manhwa", get(trending_manhwa))
        .route("/trending/manhua", get(trending_manhua))
        .route("/trending/adult-manhwa", get(trending_adult_manhwa))
        .route("/add", axum::routing::post(add))
}

// ── GET / — fan-out search ──────────────────────────────────────────────

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn search(
    claims: Claims,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<Vec<DiscoverResult>>> {
    let cfg = &state.config;
    let http = &state.http;
    let q = &query.q;
    let allow_explicit = claims.allow_explicit;

    let (mu, al, md, nu, ww, nh) = tokio::join!(
        search_mu(http, &cfg.mangaupdates_url, q),
        search_anilist(http, &cfg.anilist_url, q, allow_explicit),
        search_mangadex(http, &cfg.mangadex_meta_url, q),
        search_novelupdates(http, &cfg.plugin_host_url, q),
        search_wuxiaworld(http, &cfg.wuxiaworld_meta_url, q),
        async {
            if allow_explicit {
                search_nhentai(http, &cfg.plugin_host_url, q).await
            } else {
                anyhow::bail!("nhentai not queried — non-explicit user")
            }
        },
    );

    let mut raw = Vec::new();
    let mut any_succeeded = false;
    for v in [
        mu.as_ref(),
        al.as_ref(),
        md.as_ref(),
        nu.as_ref(),
        ww.as_ref(),
        nh.as_ref(),
    ]
    .into_iter()
    .filter_map(|r| r.ok())
    {
        any_succeeded = true;
        raw.extend(v.iter().cloned());
    }
    if !any_succeeded {
        return Err(AppError::BadGateway);
    }

    // Fire-and-forget cover-cache seeding for MU results — mirrors
    // `SeedAndCacheCoverAsync`. .NET re-fetches MU here (redundant network
    // call); reusing the already-mapped results instead, same output.
    if let Ok(mu_results) = &mu {
        for r in mu_results {
            if let Some(cover) = r.cover_url.clone() {
                let db = state.db.clone();
                let http = state.http.clone();
                let title = r.title.clone();
                let download_dir = cfg.download_dir.clone();
                let source_id = r.mangaupdates_id.clone();
                tokio::spawn(async move {
                    discover::seed_and_cache_cover(
                        &db,
                        &http,
                        &title,
                        &cover,
                        &download_dir,
                        "mangaupdates",
                        &source_id,
                    )
                    .await;
                });
            }
        }
    }

    let merged = discover::merge_fan_out(raw);
    let results = enrich_and_check_library(&state.db, &claims.user_id, merged).await?;
    Ok(Json(results))
}

async fn search_mu(
    http: &reqwest::Client,
    base: &str,
    q: &str,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let series = metadata::mangaupdates::search(http, base, q).await?;
    Ok(discover::filter_mu_scope(
        series.into_iter().map(mu_to_result).collect(),
    ))
}

async fn search_anilist(
    http: &reqwest::Client,
    endpoint: &str,
    q: &str,
    allow_explicit: bool,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let series = metadata::anilist::search(http, endpoint, q, allow_explicit).await?;
    Ok(series.into_iter().map(anilist_to_result).collect())
}

async fn search_mangadex(
    http: &reqwest::Client,
    base: &str,
    q: &str,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let series = metadata::mangadex::search(http, base, q).await?;
    Ok(series.into_iter().map(mangadex_to_result).collect())
}

async fn search_novelupdates(
    http: &reqwest::Client,
    plugin_host_url: &str,
    q: &str,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let series = metadata::novelupdates::search(http, plugin_host_url, q).await?;
    Ok(series.into_iter().map(novelupdates_to_result).collect())
}

async fn search_wuxiaworld(
    http: &reqwest::Client,
    base: &str,
    q: &str,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let series = metadata::wuxiaworld::search(http, base, q).await?;
    Ok(series.into_iter().map(wuxiaworld_to_result).collect())
}

#[derive(Deserialize)]
struct PluginSearchResult {
    id: Option<String>,
    title: Option<String>,
}

async fn search_nhentai(
    http: &reqwest::Client,
    plugin_host_url: &str,
    q: &str,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let url = format!(
        "{}/nhentai/search?q={}",
        plugin_host_url.trim_end_matches('/'),
        urlencoding::encode(q)
    );
    let resp = http.get(&url).send().await?.error_for_status()?;
    let results: Vec<PluginSearchResult> = resp.json().await?;
    Ok(results
        .into_iter()
        .map(|s| DiscoverResult {
            mangaupdates_id: s.id.unwrap_or_default(),
            title: s.title.unwrap_or_default(),
            status: "complete".to_string(),
            content_type: "hentai".to_string(),
            source: "nhentai".to_string(),
            is_explicit: true,
            ..Default::default()
        })
        .collect())
}

// ── DTO mapping ──────────────────────────────────────────────────────────

fn mu_to_result(s: metadata::mangaupdates::MuSeries) -> DiscoverResult {
    let is_explicit = s
        .tags
        .as_deref()
        .map(|t| {
            t.split(',').any(|t| {
                let t = t.trim();
                t.eq_ignore_ascii_case("adult") || t.eq_ignore_ascii_case("hentai")
            })
        })
        .unwrap_or(false);
    DiscoverResult {
        mangaupdates_id: s.series_id.to_string(),
        title: s.title,
        description: s.description,
        cover_url: s.cover_url,
        status: s.status,
        author: s.author,
        year: s.year,
        tags: s.tags,
        content_type: s.content_type,
        source: "mangaupdates".to_string(),
        is_explicit,
        ..Default::default()
    }
}

fn anilist_to_result(s: metadata::anilist::AniListSeries) -> DiscoverResult {
    DiscoverResult {
        mangaupdates_id: s.source_id,
        title: s.title,
        description: s.description,
        cover_url: s.cover_url,
        status: s.status,
        author: s.author,
        year: s.year,
        content_type: s.content_type,
        source: "anilist".to_string(),
        is_explicit: s.is_adult,
        ..Default::default()
    }
}

fn mangadex_to_result(s: metadata::mangadex::MangaDexSeries) -> DiscoverResult {
    DiscoverResult {
        mangaupdates_id: s.source_id,
        title: s.title,
        description: s.description,
        cover_url: s.cover_url,
        status: s.status,
        author: s.author,
        year: s.year,
        content_type: s.content_type,
        source: "mangadex".to_string(),
        ..Default::default()
    }
}

fn novelupdates_to_result(s: metadata::novelupdates::NovelUpdatesSeries) -> DiscoverResult {
    DiscoverResult {
        mangaupdates_id: s.source_id,
        title: s.title,
        cover_url: s.cover_url,
        status: s.status,
        content_type: "novel".to_string(),
        source: "novelupdates".to_string(),
        ..Default::default()
    }
}

fn wuxiaworld_to_result(s: metadata::wuxiaworld::WuxiaWorldSeries) -> DiscoverResult {
    DiscoverResult {
        mangaupdates_id: s.source_id,
        title: s.title,
        cover_url: s.cover_url,
        status: s.status,
        author: s.author,
        content_type: "novel".to_string(),
        source: "wuxiaworld".to_string(),
        ..Default::default()
    }
}

// ── GET /trending/{lane} ─────────────────────────────────────────────────

async fn enrich_and_check_library(
    pool: &SqlitePool,
    user_id: &str,
    mut items: Vec<DiscoverResult>,
) -> AppResult<Vec<DiscoverResult>> {
    for r in &mut items {
        let (in_library, library_id) = discover::check_in_library(
            pool,
            user_id,
            &r.source,
            &r.mangaupdates_id,
            &r.title,
            &r.content_type,
        )
        .await?;
        r.in_library = in_library;
        r.library_id = library_id;
        discover::enrich_cover(pool, r).await?;
    }
    Ok(items)
}

async fn trending_manga(
    claims: Claims,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<DiscoverResult>>> {
    const LANE: &str = "manga";
    let cached = match state.trending.get_fresh(LANE) {
        Some(c) => c,
        None => match metadata::mangaupdates::latest_releases(
            &state.http,
            &state.config.mangaupdates_url,
        )
        .await
        {
            Ok(fresh) => {
                let mapped: Vec<_> = fresh.into_iter().map(mu_to_result).collect();
                if !mapped.is_empty() {
                    state.trending.set(LANE, mapped.clone());
                    mapped
                } else {
                    state.trending.get_stale(LANE).unwrap_or_default()
                }
            }
            Err(_) => state.trending.get_stale(LANE).unwrap_or_default(),
        },
    };
    let results = enrich_and_check_library(
        &state.db,
        &claims.user_id,
        cached.into_iter().take(6).collect(),
    )
    .await?;
    Ok(Json(results))
}

async fn anilist_trending_lane(
    state: &AppState,
    lane: &str,
    country: &str,
    is_adult: bool,
) -> Vec<DiscoverResult> {
    match state.trending.get_fresh(lane) {
        Some(c) => c,
        None => {
            let fresh = metadata::anilist::trending(
                &state.http,
                &state.config.anilist_url,
                country,
                is_adult,
                15,
            )
            .await;
            let mapped: Vec<_> = fresh.into_iter().map(anilist_to_result).collect();
            if !mapped.is_empty() {
                state.trending.set(lane, mapped.clone());
                mapped
            } else {
                state.trending.get_stale(lane).unwrap_or_default()
            }
        }
    }
}

async fn trending_manhwa(
    claims: Claims,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<DiscoverResult>>> {
    let cached = anilist_trending_lane(&state, "manhwa", "KR", false).await;
    let results = enrich_and_check_library(
        &state.db,
        &claims.user_id,
        cached.into_iter().take(6).collect(),
    )
    .await?;
    Ok(Json(results))
}

async fn trending_manhua(
    claims: Claims,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<DiscoverResult>>> {
    let cached = anilist_trending_lane(&state, "manhua", "CN", false).await;
    let results = enrich_and_check_library(
        &state.db,
        &claims.user_id,
        cached.into_iter().take(6).collect(),
    )
    .await?;
    Ok(Json(results))
}

async fn trending_adult_manhwa(
    claims: Claims,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<DiscoverResult>>> {
    if !claims.allow_explicit {
        return Err(AppError::Forbidden);
    }
    let cached = anilist_trending_lane(&state, "adult-manhwa", "KR", true).await;
    let results = enrich_and_check_library(
        &state.db,
        &claims.user_id,
        cached.into_iter().take(6).collect(),
    )
    .await?;
    Ok(Json(results))
}

// ── POST /add ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AddMangaBody {
    mangaupdates_id: Option<String>,
    source: Option<String>,
    source_id: Option<String>,
    title: String,
    description: Option<String>,
    cover_url: Option<String>,
    #[serde(default = "default_status")]
    status: String,
    author: Option<String>,
    year: Option<i64>,
    tags: Option<String>,
    #[serde(default = "default_content_type")]
    content_type: String,
    is_explicit: Option<bool>,
}

fn default_status() -> String {
    "unknown".to_string()
}

fn default_content_type() -> String {
    "manga".to_string()
}

async fn add(
    claims: Claims,
    State(state): State<AppState>,
    Json(body): Json<AddMangaBody>,
) -> AppResult<Json<TitleDto>> {
    let resolved_cover = match &body.cover_url {
        Some(u) => discover::resolve_meta_cover(&state.db, u).await?,
        None => None,
    };

    let meta_source = body.source.clone().or_else(|| {
        body.mangaupdates_id
            .as_ref()
            .map(|_| "mangaupdates".to_string())
    });
    let meta_source_id = body
        .source_id
        .clone()
        .or_else(|| body.mangaupdates_id.clone());

    let mut existing = None;
    if let (Some(src), Some(sid)) = (&meta_source, &meta_source_id) {
        existing = titles::find_id_by_metadata_source(&state.db, src, sid).await?;
    }
    if existing.is_none() {
        if let Some(mu_id) = &body.mangaupdates_id {
            existing = titles::find_id_by_mangaupdates_id(&state.db, mu_id).await?;
        }
    }

    let title_id = if let Some(id) = existing {
        titles::coalesce_update_title(
            &state.db,
            &id,
            body.description.as_deref(),
            body.author.as_deref(),
            body.year,
            body.tags.as_deref(),
        )
        .await?;
        id
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        let has_adult_tag = body
            .tags
            .as_deref()
            .map(|t| t.split(',').any(|t| t.trim().eq_ignore_ascii_case("adult")))
            .unwrap_or(false);
        let is_explicit = body.is_explicit == Some(true)
            || body.content_type == "hentai"
            || discover::is_hentai_tag(body.tags.as_deref())
            || has_adult_tag;
        let clean_title =
            discover::strip_search_qualifier(&body.title).unwrap_or_else(|| body.title.clone());

        titles::insert_title(
            &state.db,
            &titles::NewTitle {
                id: &id,
                mangaupdates_id: body.mangaupdates_id.as_deref(),
                metadata_source: meta_source.as_deref(),
                metadata_source_id: meta_source_id.as_deref(),
                title: &clean_title,
                description: body.description.as_deref(),
                cover_url: resolved_cover.as_deref(),
                status: &body.status,
                author: body.author.as_deref(),
                year: body.year,
                tags: body.tags.as_deref(),
                content_type: &body.content_type,
                is_explicit,
            },
        )
        .await?;
        id
    };

    titles::insert_user_title_if_absent(&state.db, &claims.user_id, &title_id).await?;

    spawn_add_manga_background(
        &state,
        &body,
        title_id.clone(),
        meta_source,
        meta_source_id,
        resolved_cover,
    );

    let item = titles::fetch_title(&state.db, &title_id, &claims.user_id)
        .await?
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("title vanished immediately after insert"))
        })?;
    Ok(Json(TitleDto::from(item)))
}

/// Cover download + authority-routed alias refresh + source matching — port
/// of `AddManga`'s `Task.Run` background block.
fn spawn_add_manga_background(
    state: &AppState,
    body: &AddMangaBody,
    title_id: String,
    meta_source: Option<String>,
    meta_source_id: Option<String>,
    resolved_cover: Option<String>,
) {
    let db = state.db.clone();
    let http = state.http.clone();
    let plugin_host_url = state.config.plugin_host_url.clone();
    let download_dir = state.config.download_dir.clone();
    let mangaupdates_url = state.config.mangaupdates_url.clone();
    let anilist_url = state.config.anilist_url.clone();
    let mu_id = body.mangaupdates_id.clone();
    let title_name = body.title.clone();
    let content_type = body.content_type.clone();

    tokio::spawn(async move {
        titles::clear_sync_log(&db, &title_id).await.ok();

        if let Some(cover) = &resolved_cover {
            titles::append_sync_log(&db, &title_id, "Downloading cover…").await;
            discover::download_cover(&db, &http, &title_id, cover, &download_dir).await;
        }

        match meta_source.as_deref() {
            Some("mangaupdates") => {
                titles::append_sync_log(&db, &title_id, "Fetching metadata from MangaUpdates…")
                    .await;
                let id_str = meta_source_id.as_deref().or(mu_id.as_deref());
                if let Some(mu_num) = id_str.and_then(|s| s.parse::<u64>().ok()) {
                    if let Ok(Some(series)) =
                        metadata::mangaupdates::series_detail(&http, &mangaupdates_url, mu_num)
                            .await
                    {
                        if !series.associated_names.is_empty() {
                            titles::clear_title_aliases(&db, &title_id).await.ok();
                            for alias in &series.associated_names {
                                titles::insert_title_alias(&db, &title_id, alias).await.ok();
                            }
                            titles::append_sync_log(
                                &db,
                                &title_id,
                                &format!(
                                    "Loaded {} alternate title(s)",
                                    series.associated_names.len()
                                ),
                            )
                            .await;
                        }
                    }
                }
            }
            Some("anilist") => {
                titles::append_sync_log(&db, &title_id, "Fetching synonyms from AniList…").await;
                if let Some(id) = &meta_source_id {
                    let synonyms = metadata::anilist::get_synonyms(&http, &anilist_url, id).await;
                    if !synonyms.is_empty() {
                        titles::clear_title_aliases(&db, &title_id).await.ok();
                        for syn in &synonyms {
                            titles::insert_title_alias(&db, &title_id, syn).await.ok();
                        }
                        titles::append_sync_log(
                            &db,
                            &title_id,
                            &format!("Loaded {} synonym(s)", synonyms.len()),
                        )
                        .await;
                    }
                }
            }
            Some(other) => {
                titles::append_sync_log(&db, &title_id, &format!("Metadata source: {other}")).await;
            }
            None => {
                titles::append_sync_log(&db, &title_id, "Metadata source: none").await;
            }
        }

        let is_explicit_title = titles::fetch_title(&db, &title_id, "")
            .await
            .ok()
            .flatten()
            .map(|t| t.is_explicit)
            .unwrap_or(false);
        discover::match_sources(
            &db,
            &http,
            &plugin_host_url,
            &title_id,
            &title_name,
            &content_type,
            is_explicit_title,
        )
        .await;

        titles::append_sync_log(&db, &title_id, "Sync complete").await;
        let _ = titles::update_sync_status(&db, &title_id, "ready").await;
    });
}
