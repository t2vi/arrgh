# Test Coverage Plan

Strategy: four-layer pyramid (Unit → Integration → API → E2e), sequential in CI, all reporting to Allure at `/test-reports/`. See `docs/adr/0012-testing-strategy.md`.

**Frameworks**
- Web unit: `web-svelte/` — Vitest 5 + @testing-library/svelte + `allure-vitest@^3.x` (ADR 0033 F6 cutover from the React app in `web/`, deleted; v3 required here — Vitest 5's `onTestRunEnd` hook, not the v2 `onFinished` the old React app pinned)
- Server unit + integration: Rust — `#[cfg(test)]` unit tests inline per module, `server/tests/*.rs` integration tests
- API: Hurl — `.hurl` files, JUnit XML → `junit-to-allure.mjs` → Allure JSON with `layer=api`
- E2e: Playwright + allure-playwright (Docker Compose test stack + Fixture Plugin)

**TDD**: write failing test first, then implement. Red → Green → Refactor.

Legend: ✅ exists · 🟡 partial (some red TDD) · ⬜ planned · 🔴 known failing · ❌ gap (needed, not planned yet)

---

## Web — Unit (Vitest, jsdom)

### Shared components

| File | Tests | Status |
|---|---|---|
| `components/SegmentedControl` | render, onChange | ✅ |
| `components/Toggle` | render, onChange | ✅ |
| `indexHtml.test.ts` (GH #205) | `index.html` links the arrgh favicon set + manifest + theme-color and `public/` ships them; no Vite scaffold icons | ✅ |
| `detail/ContentTypeCard` (GH #211) | offers all five content types incl. hentai; picking one saves it | ✅ |
| `components/NumberStepper` | render, onChange | ✅ |
| `components/SettingRow` | render | ✅ |
| `lib/utils` (cn) | class merging | ✅ |

### Features

| Feature | File | Tests | Status |
|---|---|---|---|
| Login | `useLogin` | initial state, submit success/fail, loading cleared | ✅ |
| Library | `useLibrary` | fetch, totalPages, remove, removingId, syncing poll, sort default+setSort refetches, toggleContentType add/remove/resets page, toggleStatus add/remove, hasFilters, clearFilters, fetches with filter params, showFiltersl | ✅ |
| Library | `MangaCard` | render, remove button, is_explicit=true→18+ pill shown, is_explicit=false→no 18+ pill | ✅ |
| Discover | `useDiscover` | submit, blank guard, navigate, added tracking, source field, addingId lifecycle, addError, contentTypeFilter, filteredData, availableTypes (6 TDD ⬜) | 🟡 |
| Discover | `SearchRow` | render, is_explicit=true→18+ badge shown, is_explicit=false→no 18+ badge, tag-based inference blocked, loading state, In Library, cover; missing cover → static placeholder + missing description → nothing (no fake loading pulse, spec 028) | ✅ |
| Discover | `ContentTypeFilter` | render, hentai pill, novel pill, onChange (2 TDD ⬜) | 🟡 |
| Discover | `SearchProgress` (spec 021) | renders exactly the server-provided sources (no hard-coded list), searching→pulsing violet, found→green + count, empty→grey "no results", error/timeout→amber with title, "Searching sources…"/"Results from…" heading, skeletons only when asked | ✅ |
| Discover | `DiscoverStore` streaming (spec 021) | `sources` event → all searching; first `source` event shows results while still fetching, later ones replace; `done{ok:false}` → discovery-failed error; new submit aborts the old stream + ignores its late events (generation guard); leaving the page aborts | ✅ |
| Discover | `splitNdjson` (`lib/api.test.ts`, spec 021) | complete lines parsed, trailing partial held until the next chunk, blank lines ignored | ✅ |
| Home | `useHome` | loads trending on mount, filters in-library, trendingLoading lifecycle | ✅ |
| Home | `Cards` | render variants, title+author below cover, error→emoji, is_explicit=true→18+ pill shown (TrendingCard + LibraryCoverCard), is_explicit=false→no 18+ pill | ✅ |
| Settings | `useSettings` | load, tab defaults, save, logout | ✅ |
| Queue | `useQueue` | fetch, sort, canClear, remove+refetch | ✅ |
| Queue | `QueueRow` | render, remove btn hidden while downloading, onRemove, error | ✅ |
| Queue | `QueueRow` | progress bar shown when downloading + pages_total > 0 | ✅ |
| Queue | `QueueRow` | progress bar hidden when pages_total = 0 | ✅ |
| Queue | `QueueRow` | percentage text matches pages_downloaded/pages_total | ✅ |
| Settings | `LogsSection` | renders level selector and log table | ✅ |
| Settings | `LogsSection` | filter selector hides entries below selected level | ✅ |
| Settings | `LogsSection` | setLogLevel called when capture level changes | ✅ |
| Manga Detail | `ChapterRow` | downloaded state — BookOpen icon, no download btn | ✅ |
| Manga Detail | `ChapterRow` | pending — Queued btn, cancel calls onCancelDownload | ✅ |
| Manga Detail | `ChapterRow` | downloading — spinner, no remove btn | ✅ |
| Manga Detail | `ChapterRow` | downloading + pages_total > 0 — progress bar + % | ✅ |
| Manga Detail | `ChapterRow` | downloading + pages_total = 0 — no progress bar | ✅ |
| Manga Detail | `ChapterRow` | error state — AlertCircle shown | ✅ |
| Manga Detail | `ChapterRow` | completed — read bar at 100%, opacity-50 | ✅ |
| Manga Detail | `ChapterRow` | has_sources=false + not downloaded — no action button | ✅ |
| Reader | `ZoomControl` | renders zoom button | ✅ |
| Reader | `ZoomControl` | popover hidden initially | ✅ |
| Reader | `ZoomControl` | opens popover showing all levels (50–150%) | ✅ |
| Reader | `ZoomControl` | calls onApply with selected level and closes popover | ✅ |
| Reader | `useImageZoom` | defaults to 100 | ✅ |
| Reader | `useImageZoom` | reads stored value from localStorage | ✅ |
| Reader | `useImageZoom` | falls back to 100 for invalid stored value | ✅ |
| Reader | `useImageZoom` | apply updates state and persists to localStorage | ✅ |
| Reader | `ScrollReader` | applies saved resume position once on chapter open; later page-seen updates don't re-force scrollTop (GH #172 regression) | ✅ |
| Reader | `ReaderStore` | triggers download + `chapterDownloading` for an undownloaded, sourced chapter; clears once poll reports downloaded; never downloads/marks unavailable a sourceless chapter; skips an already-downloaded chapter (GH #175) | ✅ |
| Reader | `ReaderFooter` | paged mode Prev/Next page + chapter-boundary crossing (label/nav swap, disabled at ends); scroll/novel mode chapter buttons disabled with no adjacent chapter, navigate on click | ✅ |
| Setup | `useSetup` | starts on step 1 | ✅ |
| Setup | `useSetup` | goToStep2 advances to step 2 | ✅ |
| Setup | `useSetup` | valid token → redirects to home (setup already complete) | ✅ |
| Setup | `useSetup` | invalid/stale token → stays on setup (server wiped) | ✅ |
| Setup | `useSetup` | no token → `api.me()` never called | ✅ |
| Settings | `SourcesSection` | browse modal open/close | ✅ |
| Settings | `SourcesSection` | install plugin success → sources refetch | ✅ |
| Settings | `PluginsSection` (spec 031) | version/unknown + origin; Update when newer; disabled + reason when blocked; failure reason shown; Revert only over a bundled copy; fallback/unavailable catalog notice | ✅ |
| Settings | `SourcesSection` | add source 502 error → error message shown | ✅ |
| Settings | `SourcesSection` | toggle source calls patchSource with flipped state | ✅ |

---

## Rust Server — Unit (`#[cfg(test)]`, inline)

Framework: plain `#[test]`/`#[tokio::test]` inline in the module under test. Run with `cargo test` (or `cargo test --lib` for unit tests only). Ported from the equivalent xUnit `*LogicTests.cs`/`*Tests.cs` unit-tagged classes (ADR 0033, S1–S10); one deliberate scope cut and one mechanism change from the .NET original:

- **E-Hentai dropped as an authority** — dead code in .NET (zero live references), superseded by nhentai (`discover.rs`'s `AUTHORITY_ORDER`, `designated_authority`). `Discover.cs`'s `SearchCandidates`/`KnownNorms` (novel-title fuzzy-match helpers) were likewise unreachable and not ported.
- **`MigrationBootstrapTests.cs` → `tests/schema_bootstrap.rs`** — the mechanism changed (EF `__EFMigrationsHistory` class migrations → `sqlx migrate` + idempotent baseline SQL, S10 #132), so these are new tests for the new mechanism, not a line-for-line port. See `src/state.rs`'s `connect_db` module doc.

| Module | Covers |
|---|---|
| `auth.rs` | JWT create/verify roundtrip, wrong-secret rejection, `require_admin`, bcrypt hash roundtrip + cross-compat against a live .NET-issued hash |
| `state.rs` | `UpdateCache` get/set/clear, `PageCache` get/set, `TrendingCache` |
| `logs.rs` | Level parsing, ring buffer eviction |
| `queue.rs` | `is_allowed_explicit` |
| `settings.rs` | Numeric/bool parsing, trending clamp, reader-mode validation |
| `content.rs` (GH #163) | `chapter_format_for`: novels → text, every other content type → pages |
| `media.rs` | `detect_content_type`, `strip_jpeg_icc`, `is_image`, `root_domain_referer`, `get_chapter_page` (dir + cbz) |
| `discover.rs` | `normalize_title`, `designated_authority`, `deduplicate`, `merge_fan_out` (incl. nhentai word-boundary upgrade), `title_matches`/`levenshtein`, `strip_search_qualifier`, `is_hentai_tag`, `filter_mu_scope`, Royal Road authority order + NU-wins dedup (spec 019) ✅ |
| `metadata/*.rs` | Per-authority response mapping (MangaUpdates, AniList, MangaDex, WuxiaWorld, Royal Road — spec 019 ✅) |
| `plugins.rs` | `fetch_index` (file:// + missing-file) |
| `sources.rs` | `seed_defaults_if_empty` (10 sources), `DEFAULT_SOURCES` includes royalroad @ priority 35 (spec 019) ✅ |
| `update_checker.rs` | GitHub release JSON → `(version, html_url)` parsing |
| `api/titles.rs`'s `patch_body_tests` | `PatchBody`'s tri-state `Option<Option<T>>` parsing for `reader_mode`/`download_dir` — absent vs. explicit `null` vs. a value are all distinguishable (an improvement over .NET's `JsonElement?`, which couldn't tell "absent" from "null" cleanly; see the module's doc comment) |

---

## Rust Server — Integration (`server/tests/*.rs`)

`tower::ServiceExt::oneshot` against `arrgh_server::api::router(state)`, isolated temp-file SQLite per test (`tests/common::build_state()` runs the real `server/migrations/`, so test and production schema can't drift). Run with `cargo test`.

| File | Covers |
|---|---|
| `auth.rs`, `users.rs` | Register/login/me/status, user CRUD, role + allow_explicit gating |
| `titles.rs`, `progress.rs` | Library list/filter/sort/paginate, title detail, PATCH, sync trigger + log, continue-reading |
| `chapters.rs`, `chapter_sync.rs` | Chapter list/detail/text, plugin-host chapter fetch + dedup |
| `queue.rs` | List/filter, admin-only clear-completed, owner-or-admin remove-or-cancel |
| `downloader.rs` | Background worker: cbz/text download, multi-source priority fallback, `"downloading"` status while in flight, error messages include the failing URL, User-Agent header sent, `download_workers` honoured (2 → two items in flight at once, 1 → never more than one; GH #160) |
| `scheduler.rs` (GH #200) | Scheduled re-sync: only chapters a sync newly finds are queued (never the backlog); per-title Always/Never override the global `auto_download`; global default off; interval from `index_interval_hours`, clamped 1–24 h |
| `settings.rs`, `sources.rs` | KV settings CRUD + validation, source list/patch/delete, seeded bundled-source content types |
| `discover.rs` (Royal Road, spec 019) | Royal Road leg in search results, leg failure non-fatal, NovelUpdates wins dedup, add stores `metadata_source=royalroad` + author from plugin meta + text chapters numbered by real number, no Sync Warning when Royal Road has no match (FR-009), web-shaped add body (`mangaupdates_id` + non-MU `source`) never stored as / deduped against a MangaUpdates id ✅ |
| `chapter_sync.rs` (#173) | re-sync replaces a chapter's stale source_id when the source now reports a different one for the same chapter ✅ |
| `chapter_sync.rs` (spec 030) | re-sync with a renumbered source chapter moves the existing row (progress kept); stale duplicate from the old numbering removed; downloaded duplicate kept ✅ |
| `chapter_sync.rs` (GH #210, spec 032) | first import marks nothing new; a re-sync (two sources, one run) marks only newly found chapters `is_new` → Home "New releases" ✅ |
| `titles.rs` / `chapters.rs` (GH #211, spec 032) | admin can set `hentai`; content-type change drops source links that don't serve the new type and re-matches; chapter list 404s outside the caller's library ✅ |
| `discover.rs` (live progress, spec 021) | `GET /api/discover/stream`: `sources` first (6 for members, nhentai 7th only for explicit users), one `source` event per leg, `done` last; timed-out leg → `status:"timeout"` while others still return; all legs failed → every event `error` + `done.ok=false`; 401 without token; a fast source's event arrives before a slow source finishes (incremental body read); last event's `results` == the one-shot response; one-shot `GET /api/discover` bounded by the same per-source timeout (all hung → 502 promptly); shared HTTP client sends a default User-Agent (MangaDex 400s without one); NovelUpdates description passed through (spec 029) ✅ |
| `schema_bootstrap.rs` (spec 019) | Migration 0004 restores the royalroad source row on existing installs, no-op on empty table, idempotent ✅ |
| `plugins.rs` | Index fetch (live → image-copy fallback), admin-gated install (404/409/422/502/201, forwards id+sha256+protocol) and delete (404/403/204); spec 031: `GET /api/plugins` status merge (update available, blocked reasons, fallback/no catalog, host down), update (forwards catalog entry; 422 blocked; relays host's reason), revert (200/409/404), admin-only |
| `deploy_docs.rs` | Every env var in the deploy docs/configmap is read by the container (spec 010 FR-008, #217) |
| `media.rs` | Covered by `media.rs`'s unit tests + a manual smoke check (no dedicated integration file — no auth on this route group to exercise) |
| `logs.rs`, `version.rs` | Log buffer read + level PATCH, version + update-available reporting |
| `discover.rs` | Fan-out search across all authorities (dedup, ordering, partial-failure tolerance, nhentai upgrade), trending lanes (TTL + stale-serve), add-to-library, `match_sources` (fuzzy title match, alias match, per-source timeout/error handling, sync warnings); trending lane size follows `trending_per_source` (GH #203) |
| `schema_bootstrap.rs` | Fresh DB gets full schema; a DB missing `metadata_source`/`metadata_source_id` (pre-dates that column) gets patched; reconnecting to an already-migrated DB is a no-op |

---

## Plugin Host — Routing (`plugin-host/src/routing.test.ts`) ✅

Vitest + supertest. `createApp(registry | Map, downloadedIds?, opts?)` exported from `index.ts`; `PluginRegistry` holds a bundled and a downloaded slot per id.

| Case | Status |
|---|---|
| `onBundleChange` → loads a new bundle file, unloads it when the file is removed (spec 020, #187) | ✅ |
| importing `index.ts` under Vitest doesn't boot a real host (no EADDRINUSE with the dev stack up — spec 025, #183) | ✅ |
| `GET /plugins` → returns all loaded plugins | ✅ |
| `GET /plugins` → empty array when no plugins loaded | ✅ |
| `GET /:plugin/info` → returns info for known plugin | ✅ |
| `GET /:plugin/info` → 404 for unknown plugin | ✅ |
| `GET /:plugin/search?q=` → calls plugin.search, returns results | ✅ |
| `GET /:plugin/search` (no q) → returns `[]` | ✅ |
| `GET /:plugin/search?q=` → 502 on plugin error | ✅ |
| `GET /:plugin/search?q=` → 404 for unknown plugin | ✅ |
| `GET /:plugin/manga/:id/chapters` → calls plugin.chapters | ✅ |
| `GET /:plugin/manga/:id/chapters` → URL-decodes source id | ✅ |
| `GET /:plugin/manga/:id/chapters` → 502 on plugin error | ✅ |
| `GET /:plugin/chapter/:id/pages` → calls plugin.pages, returns URLs | ✅ |
| `GET /:plugin/chapter/:id/pages` → 404 when plugin has no pages fn (novel) | ✅ |
| `GET /:plugin/chapter/:id/pages` → 404 for unknown plugin | ✅ |
| `GET /:plugin/chapter/:id/text` → calls plugin.chapterText, returns markdown | ✅ |
| `GET /:plugin/chapter/:id/text` → 404 when plugin has no chapterText fn (manga) | ✅ |
| `POST /plugins/install` → 400 without id/url/sha256 | ✅ |
| `POST /plugins/install` (spec 031) → verified swap to `<id>.js`, persists across restart; 422 checksum mismatch / protocol too new / load failure / wrong id; 502 download failure — previous version keeps serving in every failure | ✅ |
| `compareVersions` / `pickActive` → downloaded beats bundled unless bundled is newer; unknown download still wins (spec 031 FR-008) | ✅ |
| `GET /host` → plugin protocol; `GET /plugins` → version, origin, has_bundled (spec 031) | ✅ |
| `DELETE /plugins/:id` → 403 for bundled plugin | ✅ |
| `DELETE /plugins/:id` → 204 removes community plugin; 200 reverts a download to the bundled version; removes only the file that plugin registered (#218) | ✅ |
| `rewriteCdpHost` → rewrites 0.0.0.0 to localhost for local dev | ✅ |
| Hung plugin call on any route (search/trending/meta/chapters/pages/text/cover) → 504 after `callTimeoutMs` (GH #159) | ✅ |
| `rewriteCdpHost` → rewrites internal hostname to Docker service name | ✅ |
| `rewriteCdpHost` → preserves path after host rewrite | ✅ |

## Plugin Contract — `plugin-index.json` (`plugin-host/src/contract.test.ts`) ✅

Every plugin's own `info`-shape/fn-export/fixture-behavior tests moved into its own
`t2vi/arrgh-plugin-<id>` repo (spec 031 phase C, #199) — see that repo's `test/contract.test.ts`
(the shared SDK's `pluginContractTests`, `arrgh-plugin-sdk/testing`) and `test/parse.test.ts`.
What's left here only covers cross-cutting consistency that has to live in the monorepo:

| Check | Cases | Status |
|---|---|---|
| plugin-index consistency | novelupdates index.json includes novel; royalroad bundled novel entry present (flipped from the ADR 0024 "absent" guard); manhuafast/boxnovel absent (removed sources); fixture never marked bundled | ✅ |
| every bundled plugin (spec 031 FR-010) | every index entry has `sha256`/`protocol` keys; every entry with a `download_url` has a 64-hex `sha256` (spec 031 FR-013) | ✅ |

## Plugin Bundle Fetch — image build (`scripts/fetch-plugin-bundles.mjs`) ✅ spec 031 phase D

`plugins/` contains only the e2e `fixture/` plugin now — every other plugin lives in its own repo
and is fetched from its published GitHub release at build time (`plugin-host/Dockerfile`) and by
`scripts/sync-plugins.sh` for local dev, both calling this one script. A mismatch (wrong sha256, a
bundle's `info.version` not matching the index) fails the build/fetch (FR-016). Not fixture-mocked
in CI — `.github/workflows/ci.yml`'s "Plugin Host" job runs it for real against the network as a
smoke test (catches a rotated/deleted GitHub release before merge); `docker build -f
plugin-host/Dockerfile .` is the full integration proof.

| Check | Cases | Status |
|---|---|---|
| `bundledEntries()` (`scripts/fetch-plugin-bundles.test.mjs`, `node --test`) | pure filter of `plugin-index.json`'s `bundled: true` entries; real index's bundled entries all have a `download_url` + 64-hex `sha256`, fixture excluded — no network | ✅ |
| checksum verify | download → sha256 mismatch throws, exits non-zero | ✅ (verified manually against the real releases during phase D implementation) |
| version verify | fetched bundle's own `info.version` must equal the index entry's `version`, or throws | ✅ (manual) |
| idempotent | a file already on disk whose sha256 already matches is not refetched | ✅ (manual) |

## Plugin Protocol Drift Guard (`plugin-host/src/protocol.test.ts`) ✅ spec 031 phase D

Dev-only dynamic `import('arrgh-plugin-sdk')` (plugin-host stays CommonJS at runtime — its
dynamic `require()`-based bundle loading can't take an ESM-only package as a real dependency) —
asserts plugin-host's own `PLUGIN_PROTOCOL` constant equals the SDK's, so the two can't silently
drift.

---

## API — Hurl (live server, Docker Compose stack)

Out-of-process HTTP tests against the running server. Catches: middleware ordering, JWT config, startup failures, response shapes.

**Runner**: `api-tests/run.sh` — registers admin, logs in → captures token → runs all `.hurl` files in order → converts JUnit XML to Allure JSON (`layer=api`).

| File | Scenarios | Status |
|---|---|---|
| `tests/version.hurl` | GET /api/version — shape + semver format | ✅ |
| `tests/auth.hurl` | status, login fail/success, /me unauth/badtoken/valid, PATCH /me | ✅ |
| `tests/settings.hurl` | GET settings, POST save, confirm saved | ✅ |
| `tests/titles.hurl` | list empty, pagination params, 404 on unknown, DELETE 404 | ✅ |
| `tests/sources.hurl` | add, list, patch (disable), delete, list empty again | ✅ |
| `tests/plugins.hurl` | index list, install no-url → 400, delete bundled → 403 | ✅ |
| `tests/queue.hurl` | list empty, clear completed idempotent, remove unknown → 404 | ✅ |
| `tests/logs.hurl` | GET logs, GET level, PATCH level → debug → restore info | ✅ |

**Note**: Discover endpoints excluded from API layer — calls real external APIs. Covered by Rust integration tests with mocked HTTP (`server/tests/discover.rs`).

**Local run** (requires docker-compose.test.yml stack running):
```bash
docker compose -f docker-compose.test.yml up -d --build
cd api-tests && bash run.sh
docker compose -f docker-compose.test.yml down -v
```

---

## E2e — Playwright (Docker Compose + Fixture Plugin)

See ADR 0023 for full architecture decisions (fixture server, isolation, CI, shared Allure cache).

**Infrastructure**: `docker-compose.test.yml` — replaces plugin-host with standalone Fixture Plugin server at `http://fixture:4001`. No CloakBrowser. Admin seeded via `global-setup.ts` → `POST /api/auth/register`.

**Fixture modes** (`plugins/fixture/`):

The fixture responds to `/:source/search`, `/:source/manga/:id/chapters`, `/:source/chapter/:id/pages` for **any source key** (not just `fixture`). This means `external_sources` for mangadex, toonily, asurascans etc. all resolve through the fixture in e2e.

| Title | Fixture behaviour |
|---|---|
| "Fixture Manga" | Returns 3 chapters, 3 pages (tiny JPEG at `/image.jpg`) |
| "Fixture Manhwa" | Same as Fixture Manga but searched with `content_type=manhwa` — tests non-manga chapter sync |
| "Fixture No Match" | `/search` returns `[]` → triggers Sync Warning |
| "Fixture 502" | `/manga/:id/chapters` returns HTTP 502 |
| "Fixture Empty Pages" | `/chapter/:id/pages` returns `[]` |

**Scenarios**:

| Scenario | Spec | Fixture mode | Status |
|---|---|---|---|
| Unauthenticated → redirected to login | `auth.spec.ts` | none | ✅ |
| Login → navigate → logout → redirected to login | `auth.spec.ts` | none | ✅ |
| Wrong password → error shown | `auth.spec.ts` | none | ✅ |
| Add title via API → appears in library | `library.spec.ts` | Fixture Manga | ✅ |
| Add title → sync progress overlay visible | `library.spec.ts` | Fixture Manga | ✅ |
| Source match fails → Sync Warning badge shown | `library.spec.ts` | Fixture No Match | ✅ |
| Chapters have `has_sources=true` after sync | `library.spec.ts` | Fixture Manga | ✅ |
| Manual re-sync idempotent — chapter count unchanged | `library.spec.ts` | Fixture Manga | ✅ |
| Queue chapter → download reaches `done` | `download.spec.ts` | Fixture Manga | ✅ |
| Source 502 on chapter sync → sync warning shown, status `ready` | `download.spec.ts` | Fixture 502 | ✅ |
| Empty pages → queue item shows error state | `download.spec.ts` | Fixture Empty Pages | ✅ |
| Navigate to bad title URL → error state, no crash | `discover.spec.ts` | none | ✅ |
| Discover page loads with search input | `discover.spec.ts` | none | ✅ |
| ContentTypeFilter pills appear after search | `discover.spec.ts` | Fixture Manga | ✅ |
| ContentTypeFilter pill toggles active on click | `discover.spec.ts` | Fixture Manga | ✅ |
| Add from discover search → button shows "In Library" | `discover.spec.ts` | Fixture Manga | ✅ |
| ZoomControl button visible in manga reader | `reader.spec.ts` | Fixture Manga | ✅ |
| ZoomControl dropdown shows all 5 levels (50–150%) | `reader.spec.ts` | Fixture Manga | ✅ |
| Selecting 150% sets image `max-width: 1200px` | `reader.spec.ts` | Fixture Manga | ✅ |
| Selecting 50% sets image `max-width: 400px` | `reader.spec.ts` | Fixture Manga | ✅ |
| Zoom persists via localStorage after reload | `reader.spec.ts` | Fixture Manga | ✅ |
| Zoom applies `max-width` in scroll reader | `reader.spec.ts` | Fixture Manga | ✅ |
| Manhwa title → chapters rendered after sync (fixture serves any source key) | `library.spec.ts` | Fixture Manhwa | ⬜ |
| Chapter row shows spinner + progress bar while downloading (no navigation needed) | `library.spec.ts` | Fixture Manga | ⬜ |
| Chapter row flips to "Downloaded" after download completes (no navigation needed) | `library.spec.ts` | Fixture Manga | ⬜ |
| Queue row shows `"downloading"` badge + spinner (not `"in_progress"` / clock) | `library.spec.ts` | Fixture Manga | ⬜ |

---

## Allure tagging

Every web test automatically receives `layer=UI` and `tag=Web` via `beforeEach` in `web-svelte/src/test-setup.ts`.

To tag a specific test or suite further, call inside `beforeEach` or at the top of a test:

```ts
await allure.feature('Queue')      // shows in Behaviors tab
await allure.story('remove item')
await allure.severity('critical')
await allure.owner('vinny')
```

Failure categories: `allure-categories.json` at repo root — Product defects (failed), Test defects (broken), Skipped.

Server (Rust) tests are not yet Allure-wired — see CLAUDE.md's Testing section for why (no JUnit output from `cargo test`) — so they don't appear in the Suites view.

---

## API Live Tests (`api-live-tests/`)

**Not in CI.** On-demand end-to-end validation hitting a real running server + plugin-host. Requires `API_USER`/`API_PASS` (or `API_TOKEN`). NH tests require CloakBrowser (`CLOAKBROWSER_WS_URL` set in plugin-host env).

```bash
cd api-live-tests
API_USER=vinny API_PASS=... npm test          # all tests (44 total)
API_USER=vinny API_PASS=... npm run test:update  # refresh discover snapshots
```

**File order** (enforced by `CliOrderSequencer` in `vitest.config.ts` — nhentai must run before long-running novel sync to keep CloakBrowser fresh):

| File | What it tests |
|---|---|
| `library-flow-mangadex.live.test.ts` | 8-step manga flow: Berserk via MangaDex (no CF) |
| `library-flow.live.test.ts` | 8-step hentai flow: KayaNetori via nhentai (CloakBrowser) |
| `library-flow-manhwa.live.test.ts` | 8-step manhwa flow: Solo Leveling via AsuraScans (no CF) |
| `library-flow-novel.live.test.ts` | 8-step novel flow: ISSTH via WuxiaWorld (official API); text endpoint; 120s sync timeout |
| `library-flow-royalroad.live.test.ts` | 8-step English-original flow: The Primal Hunter via Royal Road (authority + source); chapter 1389 keeps its number; text endpoint |
| `sources.live.test.ts` | Sources list snapshot + nhentai=hentai assertion |
| `discover.live.test.ts` | Discover search snapshots per content type (snapshot; update periodically) |

**Library flow steps** (all 6 flows follow same 8-step pattern):

| Step | What |
|---|---|
| 1 | discover returns a result for the content type |
| 2 | add to library |
| 3 | sync_status → ready |
| 4 | chapters loaded with has_sources=true |
| 5 | sync log contains source name + "Sync complete" |
| 6 | queue chapter 1 for download |
| 7 | download completes (status=done) |
| 8 | chapter viewable (image 200 for manga/manhwa/hentai; text endpoint for novel) |

| Flow | Authority | Status |
|---|---|---|
| manga | MangaDex | ✅ |
| hentai | nhentai | ✅ |
| manhwa | AsuraScans | ✅ |
| novel | WuxiaWorld | ✅ |

---

## Live Source Snapshot Tests (`live-tests/`)

**Not in CI.** Run on-demand when a source-related bug is suspected or to update fixtures after a source layout change. See ADR 0033.

```bash
# First run — generates snapshots (no prior .snap files exist)
cd live-tests && npm install && npm test

# After a source changes — update snapshots, inspect diff, update behavior test fixtures
cd live-tests && npm run test:update

# Single source
cd live-tests && npx vitest run src/asurascans.live.test.ts

# CF-protected sources require CloakBrowser running
CLOAK_WS_URL=ws://localhost:3000 npm test
```

**What runs per source**: search → snapshot parsed results + save raw response. Chains to chapters → pages (or chapterText for novels). CF sources (asurascans, toonily, nhentai, manhuafast, boxnovel, manga18fx, novelfull, novelupdates) skip when `CLOAK_WS_URL` not set.

**Corpus**: `live-tests/corpus/<source>.json` — adversarial titles chosen for known edge cases (hyphens in slugs, `(Novel)` suffix, special characters, apostrophes). Add new entries whenever a real parsing bug is found.

**Raw snapshots**: `live-tests/snapshots/<source>/` — HTML/JSON files saved by `captureFetch` (non-CF) or CloakBrowser page capture (CF). These are the ground truth for updating behavior test fixtures in `plugin-host/src/`.

**Parsed snapshots**: Vitest `.snap` files in `live-tests/src/__snapshots__/`. Diff tells you what the parser returns now vs. before.

| Source | CF? | Operations | Status |
|---|---|---|---|
| asurascans | ✅ | search, chapters, pages | ⬜ (run to generate) |
| mangadex | — | search, chapters, pages | ⬜ |
| mangapill | — | search, chapters, pages | ⬜ |
| toonily | ✅ | search, chapters, pages | ⬜ |
| novelfull | ✅ | search, meta, chapters, chapterText | ⬜ |
| nhentai | ✅ | search, chapters, pages | ⬜ |
| manga18fx | ✅ | search, chapters, pages | ✅ |
| novelupdates | ✅ | search | ⬜ |

---

## Running tests

```bash
# Rust server — unit (#[cfg(test)]) + integration together
cd server && cargo test
cd server && cargo clippy --all-targets
cd server && cargo fmt --check

# Web
cd web-svelte && npm test
cd web-svelte && npm run test:coverage   # with coverage

# Plugin host (routing + plugin contract)
cd plugin-host && npm test

# API tests — layer 3 (requires Docker stack)
docker compose -f docker-compose.test.yml up -d --build
cd api-tests && bash run.sh
docker compose -f docker-compose.test.yml down -v

# E2e — layer 4 (requires Docker stack, run after API tests)
docker compose -f docker-compose.test.yml up -d --build
cd e2e && npm ci && npx playwright install chromium --with-deps && npm test
docker compose -f docker-compose.test.yml down -v
```
