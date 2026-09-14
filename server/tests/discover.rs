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

// ── GET /api/discover/trending/* ────────────────────────────────────────

#[tokio::test]
async fn trending_manga_unauthorized() {
    let mock = common::start_mock_plugin_host("{}", false).await;
    let app = arrgh_server::api::router(common::build_discover_state(&mock).await);
    let (status, _) = send(&app, "GET", "/api/discover/trending/manga", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

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

#[tokio::test]
async fn trending_manga_caps_at_six() {
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
    assert_eq!(body.as_array().unwrap().len(), 6);
}

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

// ── match_sources via add's background task ──────────────────────────────

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
