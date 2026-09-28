//! S6 parity check for `/api/discover`. Mirrors `server-tests/DiscoverTests.cs`,
//! `DiscoverFanOutTests.cs`, and `TrendingLaneTests.cs` (ADR 0033, S6 #128).
//! Pure logic (dedup/merge/nhentai-upgrade/title-matching) is covered by
//! `src/discover.rs`'s own unit tests, not repeated here.
//!
//! `reqwest::Client` can't be intercepted by host like .NET's fake
//! `HttpMessageHandler` was, so every metadata-authority base URL in
//! `Config` is overridden to point at one local mock server per test
//! (`common::build_discover_state`) — see that helper's doc comment for why
//! pointing all four at one mock is safe (each client only recognizes its
//! own top-level JSON key; the rest see an unrecognized shape and succeed
//! empty rather than erroring).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tower::ServiceExt; // oneshot

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    let req = if let Some(b) = body {
        builder
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&b).unwrap()))
            .unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

const MU_BODY: &str = r#"{
  "results": [
    { "record": {
        "series_id": "555",
        "title": "Solo Leveling",
        "type": "Manga",
        "status": "Complete",
        "genres": [{ "genre": "Adult" }]
    } }
  ]
}"#;

// ── GET /api/discover ───────────────────────────────────────────────────

#[tokio::test]
async fn search_unauthorized_no_token() {
    let mock = common::start_mock_plugin_host("{}", false).await;
    let app = arrgh_server::api::router(common::build_discover_state(&mock).await);
    let (status, _) = send(&app, "GET", "/api/discover?q=x", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// spec: 004/FR-003
#[tokio::test]
async fn search_returns_502_when_all_authorities_fail() {
    let mock = common::start_mock_plugin_host("boom", true).await; // 500 everywhere
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await; // explicit → nhentai queried too, also fails
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "GET", "/api/discover?q=x", Some(&token), None).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

// spec: 004/FR-001
#[tokio::test]
async fn search_returns_mu_mapped_results() {
    let mock = common::start_mock_plugin_host(MU_BODY, false).await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, body) = send(&app, "GET", "/api/discover?q=solo", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    let mu = arr
        .iter()
        .find(|r| r["source"] == "mangaupdates")
        .expect("mu result present");
    assert_eq!(mu["mangaupdates_id"], "555");
    assert_eq!(mu["title"], "Solo Leveling");
    assert_eq!(mu["content_type"], "manga");
    assert_eq!(mu["is_explicit"], true); // "Adult" genre tag
    assert_eq!(mu["in_library"], false);
}

// spec: 004/FR-008
#[tokio::test]
async fn search_in_library_true_when_already_added() {
    let mock = common::start_mock_plugin_host(MU_BODY, false).await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Solo Leveling", true).await;
    common::set_mangaupdates_id(&state, &t, "555").await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/discover?q=solo", Some(&token), None).await;
    let mu = body
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source"] == "mangaupdates")
        .unwrap();
    assert_eq!(mu["in_library"], true);
    assert_eq!(mu["library_id"], t);
}

// spec: 004/FR-002
#[tokio::test]
async fn search_nhentai_not_included_for_non_explicit_user() {
    let nh_body = r#"[{"id":"nh-1","title":"Doujin"}]"#;
    let mock = common::start_mock_plugin_host(nh_body, false).await;
    let state = common::build_discover_state(&mock).await;
    let member = common::seed_user(&state, "member1", "member", false).await; // allow_explicit=false
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/discover?q=doujin", Some(&token), None).await;
    assert!(body
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["source"] != "nhentai"));
}

// spec: 004/FR-002
#[tokio::test]
async fn search_nhentai_included_for_explicit_user() {
    let nh_body = r#"[{"id":"nh-1","title":"Doujin"}]"#;
    let mock = common::start_mock_plugin_host(nh_body, false).await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await; // allow_explicit=true
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/discover?q=doujin", Some(&token), None).await;
    let nh = body
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source"] == "nhentai");
    assert!(nh.is_some());
    assert_eq!(nh.unwrap()["content_type"], "hentai");
}

#[tokio::test]
async fn search_nhentai_result_mapped_correctly() {
    // The nhentai-upgrade merge pass itself (MU manga result + matching
    // nhentai entry → single hentai result) is covered end-to-end by
    // `discover::tests::merge_fan_out_upgrades_matching_explicit_manga_to_hentai`
    // — a real HTTP integration equivalent would need MU and nhentai to
    // return from the same mock body in two incompatible shapes at once, so
    // this just confirms nhentai's own result mapping over the wire.
    //
    // NovelUpdates also proxies plugin-host with a compatible bare-array
    // shape, so it "succeeds" against this same mock too (as `content_type:
    // "novel"`) — both entries survive `merge_fan_out` (different
    // content_type ⇒ not deduped), so the assertion below filters by
    // `source`, not just `title`.
    let nh_body = r#"[{"id":"nh-1","title":"Kayanetori"}]"#;
    let mock = common::start_mock_plugin_host(nh_body, false).await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(
        &app,
        "GET",
        "/api/discover?q=kayanetori",
        Some(&token),
        None,
    )
    .await;
    let nh = body
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["title"] == "Kayanetori" && r["source"] == "nhentai")
        .unwrap();
    assert_eq!(nh["source"], "nhentai");
    assert_eq!(nh["content_type"], "hentai");
    assert_eq!(nh["is_explicit"], true);
}

// spec: 004/FR-012
#[tokio::test]
async fn search_excludes_adult_content_from_anilist_query_for_non_explicit_users() {
    let (mock, recorded) = common::start_recording_mock(&[]).await;
    let state = common::build_discover_state(&mock).await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    send(&app, "GET", "/api/discover?q=test", Some(&token), None).await;

    let seen = recorded.lock().unwrap();
    let anilist_call = seen
        .iter()
        .find(|(_, _, body)| {
            body.get("variables")
                .and_then(|v| v.get("isAdult"))
                .is_some()
        })
        .expect("anilist leg was called");
    assert_eq!(anilist_call.2["variables"]["isAdult"], false);
}

// ── GET /api/discover/trending/* ────────────────────────────────────────

#[tokio::test]
async fn trending_manga_unauthorized() {
    let mock = common::start_mock_plugin_host("{}", false).await;
    let app = arrgh_server::api::router(common::build_discover_state(&mock).await);
    let (status, _) = send(&app, "GET", "/api/discover/trending/manga", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// spec: 004/FR-010
#[tokio::test]
async fn trending_manga_empty_when_fails_and_no_cache() {
    let mock = common::start_mock_plugin_host("boom", true).await; // MU 500s
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, body) = send(
        &app,
        "GET",
        "/api/discover/trending/manga",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}

// spec: 004/FR-010
#[tokio::test]
async fn trending_manga_serves_stale_cache_when_fails() {
    let mock = common::start_mock_plugin_host("boom", true).await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;

    state.trending.set(
        "manga",
        vec![arrgh_server::discover::DiscoverResult {
            mangaupdates_id: "1".into(),
            title: "Cached Title".into(),
            status: "ongoing".into(),
            content_type: "manga".into(),
            source: "mangaupdates".into(),
            ..Default::default()
        }],
    );
    state.trending.expire_for_test("manga");

    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(
        &app,
        "GET",
        "/api/discover/trending/manga",
        Some(&token),
        None,
    )
    .await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["title"], "Cached Title");
}

// spec: 007/FR-001c, 004/FR-009
#[tokio::test]
async fn trending_lane_size_follows_trending_per_source() {
    let mock = common::start_mock_plugin_host("boom", true).await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;

    let cached: Vec<_> = (0..10)
        .map(|i| arrgh_server::discover::DiscoverResult {
            mangaupdates_id: i.to_string(),
            title: format!("Title {i}"),
            status: "ongoing".into(),
            content_type: "manga".into(),
            source: "mangaupdates".into(),
            ..Default::default()
        })
        .collect();
    state.trending.set("manga", cached);
    let db = state.db.clone();

    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);
    let lane_len = |app: axum::Router, token: String| async move {
        let (_, body) = send(
            &app,
            "GET",
            "/api/discover/trending/manga",
            Some(&token),
            None,
        )
        .await;
        body.as_array().unwrap().len()
    };

    assert_eq!(
        lane_len(app.clone(), token.clone()).await,
        5,
        "default trending_per_source"
    );
    arrgh_server::settings::set(&db, arrgh_server::settings::TRENDING_PER_SOURCE, "3")
        .await
        .unwrap();
    assert_eq!(lane_len(app, token).await, 3);
}

// spec: 004/FR-011
#[tokio::test]
async fn trending_adult_manhwa_forbidden_for_non_explicit_user() {
    let mock = common::start_mock_plugin_host("{}", false).await;
    let state = common::build_discover_state(&mock).await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "GET",
        "/api/discover/trending/adult-manhwa",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ── POST /api/discover/add ───────────────────────────────────────────────

#[tokio::test]
async fn add_unauthorized_no_token() {
    let mock = common::start_mock_plugin_host("[]", false).await;
    let app = arrgh_server::api::router(common::build_discover_state(&mock).await);
    let (status, _) = send(
        &app,
        "POST",
        "/api/discover/add",
        None,
        Some(json!({ "title": "X" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// spec: 004/FR-019
#[tokio::test]
async fn add_creates_title_and_strips_qualifier() {
    let mock = common::start_mock_plugin_host("[]", false).await; // plugin-host: no sources match
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (status, body) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({ "title": "Naruto (Manga)", "content_type": "manga" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["title"], "Naruto");

    let owned: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_titles WHERE user_id = ?")
        .bind(&admin.id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(owned, 1);
}

// spec: 004/FR-014
#[tokio::test]
async fn add_dedup_by_existing_mangaupdates_id() {
    let mock = common::start_mock_plugin_host("[]", false).await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let body = json!({ "mangaupdates_id": "777", "title": "Bleach", "content_type": "manga" });
    let (_, first) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(body.clone()),
    )
    .await;
    let (_, second) = send(&app, "POST", "/api/discover/add", Some(&token), Some(body)).await;
    assert_eq!(first["id"], second["id"]);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM titles WHERE mangaupdates_id = '777'")
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn add_with_source_anilist_stores_metadata_source() {
    let mock = common::start_mock_plugin_host("[]", false).await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (_, body) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({ "source": "anilist", "source_id": "101517", "title": "Solo Leveling", "content_type": "manhwa" })),
    )
    .await;
    let id = body["id"].as_str().unwrap().to_string();

    let (meta_source, meta_source_id, mu_id): (Option<String>, Option<String>, Option<String>) =
        sqlx::query_as(
            "SELECT metadata_source, metadata_source_id, mangaupdates_id FROM titles WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(meta_source.as_deref(), Some("anilist"));
    assert_eq!(meta_source_id.as_deref(), Some("101517"));
    assert_eq!(mu_id, None);
}

// spec: 004/FR-018
#[tokio::test]
async fn add_with_hentai_tag_sets_is_explicit() {
    let mock = common::start_mock_plugin_host("[]", false).await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (_, body) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({ "title": "Doujin", "content_type": "manga", "tags": "action,hentai" })),
    )
    .await;
    let id = body["id"].as_str().unwrap().to_string();

    let is_explicit: bool = sqlx::query_scalar("SELECT is_explicit FROM titles WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert!(is_explicit);
}

// spec: 004/FR-013
#[tokio::test]
async fn resolve_meta_cover_passes_through_a_plain_url_and_resolves_a_cached_key() {
    let state = common::build_state().await;

    let plain =
        arrgh_server::discover::resolve_meta_cover(&state.db, "https://cdn.example/cover.jpg")
            .await
            .unwrap();
    assert_eq!(plain.as_deref(), Some("https://cdn.example/cover.jpg"));

    let missing =
        arrgh_server::discover::resolve_meta_cover(&state.db, "/api/media/meta-cover?key=nope")
            .await
            .unwrap();
    assert_eq!(missing, None);

    let key = arrgh_server::discover::normalize_title("CachedCoverTitle");
    sqlx::query(
        "INSERT INTO title_meta (title_key, cover_cdn_url, fetched_at, source, source_id, chapter_count) \
         VALUES (?, 'https://cdn.example/original.jpg', '2026-01-01T00:00:00', 'mangaupdates', '1', 0)",
    )
    .bind(&key)
    .execute(&state.db)
    .await
    .unwrap();
    let resolved = arrgh_server::discover::resolve_meta_cover(
        &state.db,
        &format!("/api/media/meta-cover?key={key}"),
    )
    .await
    .unwrap();
    assert_eq!(
        resolved.as_deref(),
        Some("https://cdn.example/original.jpg")
    );
}

// spec: 004/FR-014
#[tokio::test]
async fn add_coalesces_only_null_fields_without_overwriting_existing() {
    let mock = common::start_mock_plugin_host("[]", false).await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (_, first) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({
            "mangaupdates_id": "999",
            "title": "Coalesce Test",
            "content_type": "manga",
            "author": "Original Author"
        })),
    )
    .await;
    let id = first["id"].as_str().unwrap().to_string();

    send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({
            "mangaupdates_id": "999",
            "title": "Coalesce Test",
            "content_type": "manga",
            "author": "Different Author",
            "description": "Filled in description"
        })),
    )
    .await;

    let (author, description): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT author, description FROM titles WHERE id = ?")
            .bind(&id)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(author.as_deref(), Some("Original Author"));
    assert_eq!(description.as_deref(), Some("Filled in description"));
}

// spec: 004/FR-015
#[tokio::test]
async fn add_downloads_cover_to_local_storage_in_background() {
    let mock = common::start_mock_routes(&[("/cover.jpg", 200, "fake-image-bytes")]).await;
    let tmp = std::env::temp_dir().join(format!("arrgh-covers-{}", uuid::Uuid::new_v4()));
    let state = common::build_downloader_state(&mock, tmp.to_str().unwrap()).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (_, body) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({
            "title": "Cover Download Test",
            "content_type": "manga",
            "cover_url": format!("{mock}/cover.jpg")
        })),
    )
    .await;
    let id = body["id"].as_str().unwrap().to_string();
    wait_ready(&state, &id).await;

    let cover_url: String = sqlx::query_scalar("SELECT cover_url FROM titles WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert!(cover_url.contains("_covers"));
    assert!(cover_url.ends_with(&format!("{id}.jpg")));
    let bytes = tokio::fs::read(&cover_url).await.unwrap();
    assert_eq!(bytes, b"fake-image-bytes");
}

// spec: 004/FR-016
#[tokio::test]
async fn add_mangaupdates_source_fetches_associated_names_as_aliases() {
    let mock = common::start_mock_routes(&[(
        "/series/555",
        200,
        r#"{"associated":[{"title":"Alias One"},{"title":"Alias Two"}]}"#,
    )])
    .await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (_, body) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({
            "mangaupdates_id": "555",
            "title": "MU Alias Test",
            "content_type": "manga"
        })),
    )
    .await;
    let id = body["id"].as_str().unwrap().to_string();
    wait_ready(&state, &id).await;

    let aliases = arrgh_server::titles::list_title_aliases(&state.db, &id)
        .await
        .unwrap();
    assert_eq!(
        aliases,
        vec!["Alias One".to_string(), "Alias Two".to_string()]
    );
}

// spec: 004/FR-017
#[tokio::test]
async fn add_anilist_source_fetches_synonyms_as_aliases() {
    let mock = common::start_mock_routes(&[(
        "/",
        200,
        r#"{"data":{"Media":{"synonyms":["Alt Name One","Alt Name Two"]}}}"#,
    )])
    .await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (_, body) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({
            "source": "anilist",
            "source_id": "101517",
            "title": "AniList Alias Test",
            "content_type": "manhwa"
        })),
    )
    .await;
    let id = body["id"].as_str().unwrap().to_string();
    wait_ready(&state, &id).await;

    let aliases = arrgh_server::titles::list_title_aliases(&state.db, &id)
        .await
        .unwrap();
    assert_eq!(
        aliases,
        vec!["Alt Name One".to_string(), "Alt Name Two".to_string()]
    );
}

// ── match_sources via add's background task ──────────────────────────────

// spec: 002/FR-004
#[tokio::test]
async fn add_with_matching_external_source_creates_source_and_chapters() {
    let search_body = r#"[{"id":"mangadex-source-id","title":"Naruto"}]"#;
    let chapters_body = r#"[{"source_id":"ch-1","number":1.0},{"source_id":"ch-2","number":2.0}]"#;
    let mock = common::start_mock_source_match_host(search_body, chapters_body).await;
    let state = common::build_state_with_plugin_host(&mock).await;
    common::seed_source_with_key(&state, "MangaDex", "mangadex", "manga").await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (status, body) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({ "title": "Naruto", "content_type": "manga" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let title_id = body["id"].as_str().unwrap().to_string();

    // background task — poll for sync_status to leave "syncing"
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let status: String = sqlx::query_scalar("SELECT sync_status FROM titles WHERE id = ?")
            .bind(&title_id)
            .fetch_one(&state.db)
            .await
            .unwrap();
        if status == "ready" || std::time::Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    }

    let source_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM title_sources WHERE title_id = ?")
            .bind(&title_id)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(source_count, 1);

    let chapter_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chapters WHERE title_id = ?")
        .bind(&title_id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(chapter_count, 2);
}

// spec: 002/FR-006
#[tokio::test]
async fn add_no_source_match_sets_sync_warning() {
    let search_body = r#"[{"id":"x","title":"Completely Different Title Xyz"}]"#;
    let chapters_body = r#"[]"#;
    let mock = common::start_mock_source_match_host(search_body, chapters_body).await;
    let state = common::build_state_with_plugin_host(&mock).await;
    common::seed_source_with_key(&state, "MangaDex", "mangadex", "manga").await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (_, body) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({ "title": "Naruto", "content_type": "manga" })),
    )
    .await;
    let title_id = body["id"].as_str().unwrap().to_string();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let status: String = sqlx::query_scalar("SELECT sync_status FROM titles WHERE id = ?")
            .bind(&title_id)
            .fetch_one(&state.db)
            .await
            .unwrap();
        if status == "ready" || std::time::Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    }

    let warnings: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_warnings WHERE title_id = ?")
        .bind(&title_id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(warnings, 1);
}

// ── alias-aware source matching (spec 033/#193) ──────────────────────────

// spec: 033/FR-004
#[tokio::test]
async fn match_sources_retries_search_per_alias_when_primary_title_finds_nothing() {
    let mock = common::start_mock_search_by_query(
        &[(
            "The Primal Hunter",
            r#"[{"id":"the-primal-hunter","title":"The Primal Hunter"}]"#,
        )],
        r#"[{"source_id":"ch-1","number":1.0}]"#,
    )
    .await;
    let state = common::build_state_with_plugin_host(&mock).await;
    common::seed_source_with_key(&state, "Royal Road", "royalroad", "novel").await;
    let t = common::seed_title(&state, "The Primal Hunter in a Ruined World", false).await;
    arrgh_server::titles::insert_title_alias(&state.db, &t, "The Primal Hunter")
        .await
        .unwrap();

    arrgh_server::discover::match_sources(
        &state.db,
        &state.http,
        &state.config.plugin_host_url,
        &t,
        "The Primal Hunter in a Ruined World",
        "novel",
        false,
    )
    .await;

    let source_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM title_sources WHERE title_id = ?")
            .bind(&t)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(source_count, 1);
}

// ── Royal Road — English-original novels (ADR 0034, spec 019) ────────────

const RR_SEARCH_BODY: &str = r#"[{
  "id": "36049",
  "title": "The Primal Hunter",
  "description": "On just another normal Monday, the world changed.",
  "cover_url": "https://www.royalroadcdn.com/public/covers-full/36049-the-primal-hunter.jpg",
  "status": "ongoing",
  "author": null,
  "year": null,
  "tags": "LitRPG, Progression"
}]"#;

async fn wait_ready(state: &arrgh_server::state::AppState, title_id: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let status: String = sqlx::query_scalar("SELECT sync_status FROM titles WHERE id = ?")
            .bind(title_id)
            .fetch_one(&state.db)
            .await
            .unwrap();
        if status == "ready" || std::time::Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    }
}

#[tokio::test]
async fn search_includes_royalroad_novel_result() {
    static ROUTES: [(&str, u16, &str); 1] = [("/royalroad/search", 200, RR_SEARCH_BODY)];
    let mock = common::start_mock_routes(&ROUTES).await;
    let state = common::build_discover_state(&mock).await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state);

    let (status, body) = send(
        &app,
        "GET",
        "/api/discover?q=the%20primal%20hunter",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rr = body
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source"] == "royalroad")
        .expect("royalroad result present");
    assert_eq!(rr["mangaupdates_id"], "36049");
    assert_eq!(rr["title"], "The Primal Hunter");
    assert_eq!(rr["content_type"], "novel");
    assert_eq!(rr["is_explicit"], false);
    assert!(rr["description"]
        .as_str()
        .unwrap()
        .contains("normal Monday"));
    assert_eq!(rr["tags"], "LitRPG, Progression");
}

// spec: 004/FR-003
#[tokio::test]
async fn search_royalroad_failure_does_not_fail_request() {
    static ROUTES: [(&str, u16, &str); 2] = [
        ("/royalroad/search", 500, "boom"),
        (
            "/novelupdates/search",
            200,
            r#"[{"id":"issth","title":"I Shall Seal the Heavens"}]"#,
        ),
    ];
    let mock = common::start_mock_routes(&ROUTES).await;
    let state = common::build_discover_state(&mock).await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state);

    let (status, body) = send(&app, "GET", "/api/discover?q=issth", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert!(arr.iter().any(|r| r["source"] == "novelupdates"));
    assert!(!arr.iter().any(|r| r["source"] == "royalroad"));
}

#[tokio::test]
async fn search_dedup_prefers_novelupdates_over_royalroad() {
    static ROUTES: [(&str, u16, &str); 2] = [
        ("/royalroad/search", 200, RR_SEARCH_BODY),
        (
            "/novelupdates/search",
            200,
            r#"[{"id":"the-primal-hunter","title":"The Primal Hunter"}]"#,
        ),
    ];
    let mock = common::start_mock_routes(&ROUTES).await;
    let state = common::build_discover_state(&mock).await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(
        &app,
        "GET",
        "/api/discover?q=the%20primal%20hunter",
        Some(&token),
        None,
    )
    .await;
    let hits: Vec<&Value> = body
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["title"] == "The Primal Hunter")
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["source"], "novelupdates");
}

#[tokio::test]
async fn add_royalroad_title_stores_source_fetches_author_and_syncs_chapters() {
    static ROUTES: [(&str, u16, &str); 3] = [
        (
            "/royalroad/manga/36049/meta",
            200,
            r#"{"description":"desc","cover_url":null,"author":"Zogarth","chapter_count":2,"tags":"LitRPG"}"#,
        ),
        ("/royalroad/search", 200, RR_SEARCH_BODY),
        (
            "/royalroad/manga/36049/chapters",
            200,
            r#"[{"source_id":"fiction/36049/x/chapter/1/chapter-1","number":1,"chapter_format":"text"},
                {"source_id":"fiction/36049/x/chapter/2/chapter-1389","number":1389,"chapter_format":"text"}]"#,
        ),
    ];
    let mock = common::start_mock_routes(&ROUTES).await;
    let state = common::build_state_with_plugin_host(&mock).await;
    common::seed_source_with_key(&state, "Royal Road", "royalroad", "novel").await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state.clone());

    let (status, body) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({
            "title": "The Primal Hunter",
            "source": "royalroad",
            "source_id": "36049",
            "content_type": "novel",
            "status": "ongoing"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let title_id = body["id"].as_str().unwrap().to_string();
    wait_ready(&state, &title_id).await;

    let (src, src_id, author, mu): (String, String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT metadata_source, metadata_source_id, author, mangaupdates_id FROM titles WHERE id = ?",
    )
    .bind(&title_id)
    .fetch_one(&state.db)
    .await
    .unwrap();
    assert_eq!(src, "royalroad");
    assert_eq!(src_id, "36049");
    assert_eq!(author.as_deref(), Some("Zogarth"));
    assert_eq!(mu, None);

    let links: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM title_sources WHERE title_id = ? AND source = 'royalroad'",
    )
    .bind(&title_id)
    .fetch_one(&state.db)
    .await
    .unwrap();
    assert_eq!(links, 1);

    let numbers: Vec<f64> = sqlx::query_scalar(
        "SELECT number FROM chapters WHERE title_id = ? AND chapter_format = 'text' ORDER BY number",
    )
    .bind(&title_id)
    .fetch_all(&state.db)
    .await
    .unwrap();
    assert_eq!(numbers, vec![1.0, 1389.0]);
}

/// FR-009 regression: Royal Road returning nothing for an East Asian title
/// must not raise a Sync Warning (the ADR 0024 noise).
#[tokio::test]
async fn royalroad_no_results_adds_no_sync_warning() {
    static ROUTES: [(&str, u16, &str); 3] = [
        ("/royalroad/search", 200, "[]"),
        (
            "/novelfull/search",
            200,
            r#"[{"id":"issth","title":"I Shall Seal the Heavens"}]"#,
        ),
        ("/novelfull/manga/issth/chapters", 200, "[]"),
    ];
    let mock = common::start_mock_routes(&ROUTES).await;
    let state = common::build_state_with_plugin_host(&mock).await;
    common::seed_source_with_key(&state, "Royal Road", "royalroad", "novel").await;
    common::seed_source_with_key(&state, "NovelFull", "novelfull", "novel").await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state.clone());

    let (_, body) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({
            "title": "I Shall Seal the Heavens",
            "source": "novelupdates",
            "source_id": "issth",
            "content_type": "novel",
            "status": "complete"
        })),
    )
    .await;
    let title_id = body["id"].as_str().unwrap().to_string();
    wait_ready(&state, &title_id).await;

    let warnings: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_warnings WHERE title_id = ?")
        .bind(&title_id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(warnings, 0);
    let log: String = sqlx::query_scalar(
        "SELECT COALESCE(group_concat(message, ' | '), '') FROM sync_log WHERE title_id = ?",
    )
    .bind(&title_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or_default();
    assert!(log.contains("No results from royalroad"), "sync log: {log}");
}

/// Web-client body shape: `DiscoverStore.handleAdd` sends the authority id as
/// `mangaupdates_id` for every source. A non-MU id must not land in
/// `titles.mangaupdates_id` (refresh-metadata would fetch an unrelated MU
/// series) nor dedup against a MU title that happens to share the number.
#[tokio::test]
async fn add_royalroad_web_shape_does_not_treat_id_as_mangaupdates() {
    let mock = common::start_mock_plugin_host("[]", false).await;
    let state = common::build_state_with_plugin_host(&mock).await;
    let mu_title = common::seed_title(&state, "Some Manga", false).await;
    common::set_mangaupdates_id(&state, &mu_title, "36049").await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state.clone());

    let (status, body) = send(
        &app,
        "POST",
        "/api/discover/add",
        Some(&token),
        Some(json!({
            "mangaupdates_id": "36049",
            "source": "royalroad",
            "title": "The Primal Hunter",
            "content_type": "novel",
            "status": "ongoing"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = body["id"].as_str().unwrap().to_string();
    assert_ne!(id, mu_title, "merged into an unrelated MangaUpdates title");

    let (meta_source, meta_source_id, mu_id): (Option<String>, Option<String>, Option<String>) =
        sqlx::query_as(
            "SELECT metadata_source, metadata_source_id, mangaupdates_id FROM titles WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(meta_source.as_deref(), Some("royalroad"));
    assert_eq!(meta_source_id.as_deref(), Some("36049"));
    assert_eq!(mu_id, None);
}

/// Spec 021 US3: every leg (one-shot too) is bounded by the configured
/// per-source timeout — all legs hung → 502 promptly instead of hanging.
#[tokio::test]
async fn oneshot_bounded_by_source_timeout() {
    static ROUTES: [(&str, u16, &str); 1] = [("/royalroad/search", 200, RR_SEARCH_BODY)];
    let mock = common::start_mock_routes_slow(&ROUTES, &["*"], Duration::from_secs(2)).await;
    let state = common::build_discover_state_with_timeout(&mock, Duration::from_millis(300)).await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state);

    let started = Instant::now();
    let (status, _) = send(&app, "GET", "/api/discover?q=x", Some(&token), None).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "took {:?}",
        started.elapsed()
    );
}

// ── GET /api/discover/stream — live per-source progress (spec 021) ───────

/// Reads the NDJSON body incrementally, timestamping each line relative to
/// the request start.
async fn stream(
    app: &Router,
    uri: &str,
    token: Option<&str>,
) -> (StatusCode, Vec<(Duration, Value)>) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    let started = Instant::now();
    let res = app
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let mut body = res.into_body();
    let (mut buf, mut lines) = (String::new(), Vec::new());
    while let Some(frame) = body.frame().await {
        if let Ok(data) = frame.unwrap().into_data() {
            buf.push_str(std::str::from_utf8(&data).unwrap());
            while let Some(nl) = buf.find('\n') {
                let line: String = buf.drain(..=nl).collect();
                lines.push((
                    started.elapsed(),
                    serde_json::from_str(line.trim()).unwrap(),
                ));
            }
        }
    }
    (status, lines)
}

fn source_event<'a>(lines: &'a [(Duration, Value)], key: &str) -> &'a (Duration, Value) {
    lines
        .iter()
        .find(|(_, e)| e["type"] == "source" && e["key"] == key)
        .unwrap_or_else(|| panic!("no source event for {key}"))
}

const MEMBER_SOURCES: [&str; 6] = [
    "mangaupdates",
    "anilist",
    "mangadex",
    "novelupdates",
    "wuxiaworld",
    "royalroad",
];

#[tokio::test]
async fn stream_lists_sources_then_one_event_per_source_then_done() {
    static ROUTES: [(&str, u16, &str); 1] = [("/royalroad/search", 200, RR_SEARCH_BODY)];
    let mock = common::start_mock_routes(&ROUTES).await;
    let state = common::build_discover_state(&mock).await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state);

    let (status, lines) = stream(&app, "/api/discover/stream?q=primal", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let events: Vec<&Value> = lines.iter().map(|(_, e)| e).collect();

    assert_eq!(events[0]["type"], "sources");
    let keys: Vec<&str> = events[0]["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, MEMBER_SOURCES);
    assert_eq!(events[0]["sources"][5]["label"], "Royal Road");

    let sources: Vec<&&Value> = events.iter().filter(|e| e["type"] == "source").collect();
    assert_eq!(sources.len(), 6);
    for e in &sources {
        assert!(["found", "empty", "error", "timeout"].contains(&e["status"].as_str().unwrap()));
    }
    assert_eq!(source_event(&lines, "royalroad").1["status"], "found");
    assert_eq!(source_event(&lines, "royalroad").1["count"], 1);

    assert_eq!(events.last().unwrap()["type"], "done");
    assert_eq!(events.last().unwrap()["ok"], true);
    assert_eq!(events.len(), 8);
}

#[tokio::test]
async fn stream_explicit_user_includes_nhentai() {
    let mock = common::start_mock_routes(&[]).await;
    let state = common::build_discover_state(&mock).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, lines) = stream(&app, "/api/discover/stream?q=x", Some(&token)).await;
    let sources = lines[0].1["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 7);
    assert_eq!(sources[6]["key"], "nhentai");
    assert_eq!(
        lines.iter().filter(|(_, e)| e["type"] == "source").count(),
        7
    );
}

#[tokio::test]
async fn stream_reports_timeout_status() {
    static ROUTES: [(&str, u16, &str); 2] = [
        ("/royalroad/search", 200, RR_SEARCH_BODY),
        (
            "/novelupdates/search",
            200,
            r#"[{"id":"issth","title":"I Shall Seal the Heavens"}]"#,
        ),
    ];
    let mock =
        common::start_mock_routes_slow(&ROUTES, &["/novelupdates/search"], Duration::from_secs(2))
            .await;
    let state = common::build_discover_state_with_timeout(&mock, Duration::from_millis(300)).await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state);

    let started = Instant::now();
    let (_, lines) = stream(&app, "/api/discover/stream?q=x", Some(&token)).await;
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "took {:?}",
        started.elapsed()
    );

    let nu = &source_event(&lines, "novelupdates").1;
    assert_eq!(nu["status"], "timeout");
    assert_eq!(nu["count"], 0);
    assert_eq!(source_event(&lines, "royalroad").1["status"], "found");
    assert_eq!(lines.last().unwrap().1, json!({"type": "done", "ok": true}));
}

#[tokio::test]
async fn stream_all_failed_done_not_ok() {
    let mock = common::start_mock_plugin_host("boom", true).await; // 500 everywhere
    let state = common::build_discover_state(&mock).await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state);

    let (status, lines) = stream(&app, "/api/discover/stream?q=x", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let sources: Vec<&Value> = lines
        .iter()
        .map(|(_, e)| e)
        .filter(|e| e["type"] == "source")
        .collect();
    assert_eq!(sources.len(), 6);
    assert!(sources.iter().all(|e| e["status"] == "error"));
    assert_eq!(
        lines.last().unwrap().1,
        json!({"type": "done", "ok": false})
    );
}

#[tokio::test]
async fn stream_unauthorized() {
    let mock = common::start_mock_routes(&[]).await;
    let app = arrgh_server::api::router(common::build_discover_state(&mock).await);
    let (status, _) = send(&app, "GET", "/api/discover/stream?q=x", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn stream_fast_source_event_arrives_before_slow_source_finishes() {
    static ROUTES: [(&str, u16, &str); 2] = [
        ("/royalroad/search", 200, RR_SEARCH_BODY),
        (
            "/novelupdates/search",
            200,
            r#"[{"id":"issth","title":"I Shall Seal the Heavens"}]"#,
        ),
    ];
    let mock = common::start_mock_routes_slow(
        &ROUTES,
        &["/novelupdates/search"],
        Duration::from_millis(1500),
    )
    .await;
    let state = common::build_discover_state_with_timeout(&mock, Duration::from_secs(5)).await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state);

    let (_, lines) = stream(&app, "/api/discover/stream?q=x", Some(&token)).await;
    let (rr_at, rr) = source_event(&lines, "royalroad");
    let (nu_at, nu) = source_event(&lines, "novelupdates");
    assert!(
        *rr_at < Duration::from_secs(1),
        "royalroad arrived at {rr_at:?}"
    );
    assert!(
        *nu_at >= Duration::from_millis(1400),
        "novelupdates arrived at {nu_at:?}"
    );
    assert_eq!(rr["status"], "found");
    assert_eq!(nu["status"], "found");
}

#[tokio::test]
async fn stream_final_results_equal_one_shot() {
    static ROUTES: [(&str, u16, &str); 3] = [
        ("/series/search", 200, MU_BODY),
        ("/royalroad/search", 200, RR_SEARCH_BODY),
        (
            "/novelupdates/search",
            200,
            r#"[{"id":"the-primal-hunter","title":"The Primal Hunter"}]"#,
        ),
    ];
    let mock = common::start_mock_routes(&ROUTES).await;
    let state = common::build_discover_state(&mock).await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state);

    let (_, one_shot) = send(&app, "GET", "/api/discover?q=x", Some(&token), None).await;
    let (_, lines) = stream(&app, "/api/discover/stream?q=x", Some(&token)).await;
    let last = lines
        .iter()
        .rev()
        .find(|(_, e)| e["type"] == "source")
        .unwrap();
    assert!(one_shot.as_array().unwrap().len() >= 2);
    assert_eq!(last.1["results"], one_shot);
}

/// MangaDex answers 400 to requests without a User-Agent — the shared HTTP
/// client must send one by default (found via the spec 021 source pills).
#[tokio::test]
async fn shared_http_client_sends_a_user_agent() {
    use axum::http::HeaderMap;
    use axum::response::IntoResponse;

    let app = axum::Router::new().fallback(|headers: HeaderMap| async move {
        if headers.get("user-agent").is_some_and(|v| !v.is_empty()) {
            (
                StatusCode::OK,
                [("content-type", "application/json")],
                r#"{"result":"ok","data":[]}"#,
            )
                .into_response()
        } else {
            StatusCode::BAD_REQUEST.into_response()
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let state = common::build_discover_state(&mock).await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state);

    let (_, lines) = stream(&app, "/api/discover/stream?q=berserk", Some(&token)).await;
    assert_eq!(source_event(&lines, "mangadex").1["status"], "empty");
}

/// Spec 029: NovelUpdates Series Finder rows carry a synopsis — Discover must
/// pass it through instead of dropping it.
#[tokio::test]
async fn search_novelupdates_description_passed_through() {
    static ROUTES: [(&str, u16, &str); 1] = [(
        "/novelupdates/search",
        200,
        r#"[{"id":"reverend-insanity","title":"Reverend Insanity","description":"Human beings are the very spirit of all life.","cover_url":null,"status":"unknown"}]"#,
    )];
    let mock = common::start_mock_routes(&ROUTES).await;
    let state = common::build_discover_state(&mock).await;
    let user = common::seed_user(&state, "member", "member", false).await;
    let token = common::token_for(&user);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/discover?q=reverend", Some(&token), None).await;
    let nu = body
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source"] == "novelupdates")
        .expect("novelupdates result");
    assert_eq!(
        nu["description"],
        "Human beings are the very spirit of all life."
    );
}
