//! `/api/discover` — port of `Api/Discover.cs` (ADR 0033 S6 #128; ADR 0031
//! fan-out/dedup; ADR 0032 trending lanes). E-Hentai is dead code in .NET
//! (superseded by nhentai) and isn't ported — see `crate::metadata`'s module
//! doc.

use std::convert::Infallible;
use std::future::Future;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use futures::future::{join_all, BoxFuture};
use futures::stream::{FuturesUnordered, StreamExt};
use futures::FutureExt;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::api::titles::TitleDto;
use crate::auth::Claims;
use crate::content;
use crate::discover::{self, DiscoverResult};
use crate::error::{AppError, AppResult};
use crate::metadata;
use crate::settings;
use crate::sources;
use crate::state::AppState;
use crate::titles;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(search))
        .route("/stream", get(search_stream))
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

/// Per-source bound on every Discover search leg (spec 021). Generous on
/// purpose: nhentai legitimately takes ~20–60 s and streaming means a slow
/// source no longer delays the others — this only stops a hung one from
/// holding the search open forever. The shared `reqwest::Client` has no
/// timeout of its own.
pub const DISCOVER_SOURCE_TIMEOUT: Duration = Duration::from_secs(90);

enum LegError {
    Timeout,
    Failed(anyhow::Error),
}

async fn bounded<T>(
    limit: Duration,
    leg: impl Future<Output = anyhow::Result<T>>,
) -> Result<T, LegError> {
    match tokio::time::timeout(limit, leg).await {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(LegError::Failed(e)),
        Err(_) => Err(LegError::Timeout),
    }
}

type LegResult = Result<Vec<DiscoverResult>, LegError>;

struct Leg {
    key: &'static str,
    label: &'static str,
    fut: BoxFuture<'static, LegResult>,
}

/// The Discover fan-out, one entry per queried authority in `AUTHORITY_ORDER`.
/// nhentai is only included for explicit-enabled users.
fn legs(state: &AppState, q: &str, allow_explicit: bool) -> Vec<Leg> {
    let cfg = &state.config;
    let limit = cfg.discover_source_timeout;
    let http = || state.http.clone();
    let q = || q.to_string();
    let leg = |key, label, fut: BoxFuture<'static, anyhow::Result<Vec<DiscoverResult>>>| Leg {
        key,
        label,
        fut: bounded(limit, fut).boxed(),
    };
    let mut legs = vec![
        leg(
            "mangaupdates",
            "MangaUpdates",
            search_mu(http(), cfg.mangaupdates_url.clone(), q()).boxed(),
        ),
        leg(
            "anilist",
            "AniList",
            search_anilist(http(), cfg.anilist_url.clone(), q(), allow_explicit).boxed(),
        ),
        leg(
            "mangadex",
            "MangaDex",
            search_mangadex(http(), cfg.mangadex_meta_url.clone(), q()).boxed(),
        ),
        leg(
            "novelupdates",
            "NovelUpdates",
            search_novelupdates(http(), cfg.plugin_host_url.clone(), q()).boxed(),
        ),
        leg(
            "wuxiaworld",
            "WuxiaWorld",
            search_wuxiaworld(http(), cfg.wuxiaworld_meta_url.clone(), q()).boxed(),
        ),
        leg(
            discover::ROYALROAD,
            "Royal Road",
            search_royalroad(http(), cfg.plugin_host_url.clone(), q()).boxed(),
        ),
    ];
    if allow_explicit {
        legs.push(leg(
            "nhentai",
            "nhentai",
            search_nhentai(http(), cfg.plugin_host_url.clone(), q()).boxed(),
        ));
    }
    legs
}

/// Fire-and-forget cover-cache seeding for MU results — mirrors
/// `SeedAndCacheCoverAsync`. .NET re-fetches MU here (redundant network
/// call); reusing the already-mapped results instead, same output.
fn seed_mu_covers(state: &AppState, mu_results: &[DiscoverResult]) {
    for r in mu_results {
        if let Some(cover) = r.cover_url.clone() {
            let db = state.db.clone();
            let http = state.http.clone();
            let title = r.title.clone();
            let download_dir = state.config.download_dir.clone();
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

async fn search(
    claims: Claims,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<Vec<DiscoverResult>>> {
    let outcomes = join_all(
        legs(&state, &query.q, claims.allow_explicit)
            .into_iter()
            .map(|l| async move { (l.key, l.fut.await) }),
    )
    .await;

    let mut raw = Vec::new();
    let mut any_succeeded = false;
    for (key, outcome) in outcomes {
        if let Ok(v) = outcome {
            any_succeeded = true;
            if key == "mangaupdates" {
                seed_mu_covers(&state, &v);
            }
            raw.extend(v);
        }
    }
    if !any_succeeded {
        return Err(AppError::BadGateway);
    }

    let mut merged = discover::merge_fan_out(raw, &query.q);
    merged.extend(source_fallback_results(&state, &query.q, &merged, claims.allow_explicit).await);
    let results = enrich_and_check_library(&state.db, &claims.user_id, merged).await?;
    Ok(Json(results))
}

/// Source-fallback search (spec 036, #254): for every content type the authority fan-out came
/// back with zero results for, query that content type's configured sources directly — the same
/// list `discover::match_sources` already uses post-add — and return whatever they find, marked
/// `via_source: true`. Bounded to the empty-content-type case only (FR-002): a content type an
/// authority already answered never triggers a source query.
async fn source_fallback_results(
    state: &AppState,
    query: &str,
    already_found: &[DiscoverResult],
    allow_explicit: bool,
) -> Vec<DiscoverResult> {
    let mut out = Vec::new();
    for content_type in discover::empty_content_types(already_found, allow_explicit) {
        let include_hentai = content_type == content::MANGA && allow_explicit;
        let Ok(candidates) =
            sources::matching_for_content_type(&state.db, content_type, include_hentai).await
        else {
            continue;
        };
        if candidates.is_empty() {
            continue;
        }

        let hits =
            join_all(
                candidates.into_iter().map(|(source_key, _priority)| {
                    let http = state.http.clone();
                    let plugin_host_url = state.config.plugin_host_url.clone();
                    let q = query.to_string();
                    async move {
                        source_search(http, plugin_host_url, source_key, q, content_type).await
                    }
                }),
            )
            .await;

        let content_type_results: Vec<DiscoverResult> = hits.into_iter().flatten().collect();
        out.extend(discover::deduplicate(content_type_results));
    }
    out
}

/// One source's `GET {plugin_host}/{source}/search?q=...` leg — same URL shape and soft-fail
/// behavior (timeout/connect/non-success status/unparseable body all just contribute zero
/// results) as `discover::match_sources`'s existing post-add source search.
async fn source_search(
    http: reqwest::Client,
    plugin_host_url: String,
    source_key: String,
    q: String,
    content_type: &str,
) -> Vec<DiscoverResult> {
    let url = format!(
        "{}/{}/search?q={}",
        plugin_host_url.trim_end_matches('/'),
        source_key,
        urlencoding::encode(&q)
    );
    let Ok(resp) = http.get(&url).send().await else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let Ok(hits) = resp.json::<Vec<discover::SourceSearchHit>>().await else {
        return Vec::new();
    };
    hits.into_iter()
        .filter_map(|h| discover::source_hit_to_result(h, &source_key, content_type))
        .collect()
}

// ── GET /stream — same fan-out, NDJSON per-source progress (spec 021) ────

#[derive(Serialize)]
struct SourceInfo {
    key: &'static str,
    label: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum LegStatus {
    Found,
    Empty,
    Error,
    Timeout,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamEvent {
    Sources {
        sources: Vec<SourceInfo>,
    },
    Source {
        key: &'static str,
        status: LegStatus,
        count: usize,
        ms: u64,
        /// Full merged list so far — same shape as `GET /api/discover`.
        results: Vec<DiscoverResult>,
    },
    Done {
        ok: bool,
    },
}

async fn search_stream(
    claims: Claims,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Response {
    let legs = legs(&state, &query.q, claims.allow_explicit);
    let sources = legs
        .iter()
        .map(|l| SourceInfo {
            key: l.key,
            label: l.label,
        })
        .collect();
    let started = Instant::now();
    let mut pending: FuturesUnordered<_> = legs
        .into_iter()
        .enumerate()
        .map(|(i, l)| async move { (i, l.key, l.fut.await) })
        .collect();
    let mut slots: Vec<Option<Vec<DiscoverResult>>> = (0..pending.len()).map(|_| None).collect();

    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<String, Infallible>>();
    // A failed send means the client went away — returning drops `pending`,
    // which cancels the remaining legs.
    let send = move |e: StreamEvent| {
        let line = serde_json::to_string(&e).expect("serializable event") + "\n";
        tx.unbounded_send(Ok(line)).is_ok()
    };
    tokio::spawn(async move {
        if !send(StreamEvent::Sources { sources }) {
            return;
        }
        let mut any_succeeded = false;
        let mut results = Vec::new();
        while let Some((i, key, outcome)) = pending.next().await {
            let ms = started.elapsed().as_millis() as u64;
            let (status, count) = match &outcome {
                Ok(v) if v.is_empty() => (LegStatus::Empty, 0),
                Ok(v) => (LegStatus::Found, v.len()),
                Err(LegError::Timeout) => (LegStatus::Timeout, 0),
                Err(LegError::Failed(e)) => {
                    tracing::debug!("discover: {key} failed: {e:#}");
                    (LegStatus::Error, 0)
                }
            };
            if let Ok(v) = outcome {
                any_succeeded = true;
                if key == "mangaupdates" {
                    seed_mu_covers(&state, &v);
                }
                slots[i] = Some(v);
                // Leg order, not completion order, so the settled list is
                // identical to the one-shot endpoint's.
                let raw = slots.iter().flatten().flatten().cloned().collect();
                match enrich_and_check_library(
                    &state.db,
                    &claims.user_id,
                    discover::merge_fan_out(raw, &query.q),
                )
                .await
                {
                    Ok(r) => results = r,
                    Err(e) => tracing::warn!("discover stream: library check failed: {e:?}"),
                }
            }
            let event = StreamEvent::Source {
                key,
                status,
                count,
                ms,
                results: results.clone(),
            };
            if !send(event) {
                return;
            }
        }
        send(StreamEvent::Done { ok: any_succeeded });
    });

    (
        [
            (header::CONTENT_TYPE, "application/x-ndjson"),
            (header::HeaderName::from_static("x-accel-buffering"), "no"),
        ],
        Body::from_stream(rx),
    )
        .into_response()
}

async fn search_mu(
    http: reqwest::Client,
    base: String,
    q: String,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let series = metadata::mangaupdates::search(&http, &base, &q).await?;
    Ok(discover::filter_mu_scope(
        series.into_iter().map(mu_to_result).collect(),
    ))
}

async fn search_anilist(
    http: reqwest::Client,
    endpoint: String,
    q: String,
    allow_explicit: bool,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let series = metadata::anilist::search(&http, &endpoint, &q, allow_explicit).await?;
    Ok(series.into_iter().map(anilist_to_result).collect())
}

async fn search_mangadex(
    http: reqwest::Client,
    base: String,
    q: String,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let series = metadata::mangadex::search(&http, &base, &q).await?;
    Ok(series.into_iter().map(mangadex_to_result).collect())
}

async fn search_novelupdates(
    http: reqwest::Client,
    plugin_host_url: String,
    q: String,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let series = metadata::novelupdates::search(&http, &plugin_host_url, &q).await?;
    Ok(series.into_iter().map(novelupdates_to_result).collect())
}

async fn search_wuxiaworld(
    http: reqwest::Client,
    base: String,
    q: String,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let series = metadata::wuxiaworld::search(&http, &base, &q).await?;
    Ok(series.into_iter().map(wuxiaworld_to_result).collect())
}

async fn search_royalroad(
    http: reqwest::Client,
    plugin_host_url: String,
    q: String,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let series = metadata::royalroad::search(&http, &plugin_host_url, &q).await?;
    Ok(series.into_iter().map(royalroad_to_result).collect())
}

#[derive(Deserialize)]
struct PluginSearchResult {
    id: Option<String>,
    title: Option<String>,
}

async fn search_nhentai(
    http: reqwest::Client,
    plugin_host_url: String,
    q: String,
) -> anyhow::Result<Vec<DiscoverResult>> {
    let url = format!(
        "{}/nhentai/search?q={}",
        plugin_host_url.trim_end_matches('/'),
        urlencoding::encode(&q)
    );
    let resp = http.get(&url).send().await?.error_for_status()?;
    let results: Vec<PluginSearchResult> = resp.json().await?;
    Ok(results
        .into_iter()
        .map(|s| DiscoverResult {
            mangaupdates_id: s.id.unwrap_or_default(),
            title: s.title.unwrap_or_default(),
            status: "complete".to_string(),
            content_type: content::HENTAI.to_string(),
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
        description: s.description,
        cover_url: s.cover_url,
        status: s.status,
        content_type: content::NOVEL.to_string(),
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
        content_type: content::NOVEL.to_string(),
        source: "wuxiaworld".to_string(),
        ..Default::default()
    }
}

fn royalroad_to_result(s: metadata::royalroad::RoyalRoadSeries) -> DiscoverResult {
    DiscoverResult {
        mangaupdates_id: s.source_id,
        title: s.title,
        description: s.description,
        cover_url: s.cover_url,
        status: s.status,
        tags: s.tags,
        content_type: content::NOVEL.to_string(),
        source: discover::ROYALROAD.to_string(),
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
    const LANE: &str = content::MANGA;
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
        cached
            .into_iter()
            .take(settings::trending_per_source(&state.db).await)
            .collect(),
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
    let cached = anilist_trending_lane(&state, content::MANHWA, "KR", false).await;
    let results = enrich_and_check_library(
        &state.db,
        &claims.user_id,
        cached
            .into_iter()
            .take(settings::trending_per_source(&state.db).await)
            .collect(),
    )
    .await?;
    Ok(Json(results))
}

async fn trending_manhua(
    claims: Claims,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<DiscoverResult>>> {
    let cached = anilist_trending_lane(&state, content::MANHUA, "CN", false).await;
    let results = enrich_and_check_library(
        &state.db,
        &claims.user_id,
        cached
            .into_iter()
            .take(settings::trending_per_source(&state.db).await)
            .collect(),
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
        cached
            .into_iter()
            .take(settings::trending_per_source(&state.db).await)
            .collect(),
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
    content::MANGA.to_string()
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

    // The web client sends every authority's id as `mangaupdates_id`; only
    // treat it as one when the source actually is MangaUpdates, or a Royal
    // Road/AniList numeric id collides with an unrelated MU series.
    let mu_id = body
        .mangaupdates_id
        .clone()
        .filter(|_| meta_source.as_deref() == Some("mangaupdates"));

    let mut existing = None;
    if let (Some(src), Some(sid)) = (&meta_source, &meta_source_id) {
        existing = titles::find_id_by_metadata_source(&state.db, src, sid).await?;
    }
    if existing.is_none() {
        if let Some(mu_id) = &mu_id {
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
            || body.content_type == content::HENTAI
            || discover::is_hentai_tag(body.tags.as_deref())
            || has_adult_tag;
        let clean_title =
            discover::strip_search_qualifier(&body.title).unwrap_or_else(|| body.title.clone());

        titles::insert_title(
            &state.db,
            &titles::NewTitle {
                id: &id,
                mangaupdates_id: mu_id.as_deref(),
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
            Some(discover::ROYALROAD) => {
                titles::append_sync_log(&db, &title_id, "Fetching metadata from Royal Road…").await;
                if let Some(id) = &meta_source_id {
                    if let Ok(meta) = metadata::royalroad::meta(&http, &plugin_host_url, id).await {
                        titles::coalesce_update_title(
                            &db,
                            &title_id,
                            meta.description.as_deref(),
                            meta.author.as_deref(),
                            None,
                            meta.tags.as_deref(),
                        )
                        .await
                        .ok();
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
        let _ = titles::update_sync_status(&db, &title_id, titles::SYNC_READY).await;
    });
}
