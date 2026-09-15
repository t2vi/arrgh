# Test Coverage Plan

Strategy: four-layer pyramid (Unit → Integration → API → E2e), sequential in CI, all reporting to Allure at `/test-reports/`. See `docs/adr/0012-testing-strategy.md`.

**Frameworks**
- Web unit: Vitest + @testing-library/react + `allure-vitest@^2.x` (must stay v2 — v3 incompatible with vitest v2)
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
| Discover | `SearchRow` | render, is_explicit=true→18+ badge shown, is_explicit=false→no 18+ badge, tag-based inference blocked, loading state, In Library, cover/skeleton | ✅ |
| Discover | `ContentTypeFilter` | render, hentai pill, novel pill, onChange (2 TDD ⬜) | 🟡 |
| Discover | `SearchProgress` | searching: heading, skeletons, pills stagger in; completed: "Results from…" heading, no skeletons, all pills visible immediately, matched→green after stagger, unmatched→dimmed, dot green+no-pulse | ✅ |
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
| Setup | `useSetup` | starts on step 1 | ✅ |
| Setup | `useSetup` | goToStep2 advances to step 2 | ✅ |
| Setup | `useSetup` | valid token → redirects to home (setup already complete) | ✅ |
| Setup | `useSetup` | invalid/stale token → stays on setup (server wiped) | ✅ |
| Setup | `useSetup` | no token → `api.me()` never called | ✅ |
| Settings | `SourcesSection` | browse modal open/close | ✅ |
| Settings | `SourcesSection` | install plugin success → sources refetch | ✅ |
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
| `media.rs` | `detect_content_type`, `strip_jpeg_icc`, `is_image`, `root_domain_referer`, `get_chapter_page` (dir + cbz) |
| `discover.rs` | `normalize_title`, `designated_authority`, `deduplicate`, `merge_fan_out` (incl. nhentai word-boundary upgrade), `title_matches`/`levenshtein`, `strip_search_qualifier`, `is_hentai_tag`, `filter_mu_scope` |
| `metadata/*.rs` | Per-authority response mapping (MangaUpdates, AniList, MangaDex, WuxiaWorld) |
| `plugins.rs` | `fetch_index` (file:// + missing-file) |
| `sources.rs` | `seed_defaults_if_empty` |
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
| `downloader.rs` | Background worker: cbz/text download, multi-source priority fallback, `"downloading"` status while in flight, error messages include the failing URL, User-Agent header sent |
| `settings.rs`, `sources.rs` | KV settings CRUD + validation, source list/patch/delete, seeded bundled-source content types |
| `plugins.rs` | Index fetch, admin-gated install (404/409/422/502/201) and delete (404/403/204) |
| `media.rs` | Covered by `media.rs`'s unit tests + a manual smoke check (no dedicated integration file — no auth on this route group to exercise) |
| `logs.rs`, `version.rs` | Log buffer read + level PATCH, version + update-available reporting |
| `discover.rs` | Fan-out search across all authorities (dedup, ordering, partial-failure tolerance, nhentai upgrade), trending lanes (TTL + stale-serve), add-to-library, `match_sources` (fuzzy title match, alias match, per-source timeout/error handling, sync warnings) |
| `schema_bootstrap.rs` | Fresh DB gets full schema; a DB missing `metadata_source`/`metadata_source_id` (pre-dates that column) gets patched; reconnecting to an already-migrated DB is a no-op |

---

## Plugin Host — Routing (`plugin-host/src/routing.test.ts`) ✅

Vitest + supertest. `createApp(plugins, communityIds?)` exported from `index.ts`.

| Case | Status |
|---|---|
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
| `POST /plugins/install` → 400 when url missing | ✅ |
| `DELETE /plugins/:id` → 403 for bundled plugin | ✅ |
| `DELETE /plugins/:id` → 204 removes community plugin | ✅ |
| `rewriteCdpHost` → rewrites 0.0.0.0 to localhost for local dev | ✅ |
| `rewriteCdpHost` → rewrites internal hostname to Docker service name | ✅ |
| `rewriteCdpHost` → preserves path after host rewrite | ✅ |

## Plugin Contract — Existing (`plugin-host/src/contract.test.ts`) ✅

Tests `info` shape and exported fn signatures for all bundled default plugins. No HTTP calls.

| Plugin | Cases | Status |
|---|---|---|
| mangadex | id, default_explicit=false, content_types (manga+manhwa+manhua+one-shot), fn exports | ✅ |
| mangapill | id, default_explicit=false, content_types (manga), fn exports | ✅ |
| nhentai | id, default_explicit=true, fn exports | ✅ |
| novelfull | id, default_explicit=false, content_types (novel), chapterText export | ✅ |
| royalroad | id, default_explicit=false, content_types (novel), chapterText export | ✅ |
| toonily | id, default_explicit=false, content_types (manhwa), fn exports | ✅ |
| plugin-index consistency | mangadex index.json includes manga+manhwa+manhua+one-shot | ✅ |

## Plugin Contract — New Plugins (`plugin-host/src/contract.new-plugins.test.ts`) ✅ ADR 0031

| Plugin | content_types | Novel? | Status |
|---|---|---|---|
| mangafire | `['manga','manhwa','manhua','one-shot']` | no (pages) | ✅ |
| asurascans | `['manhwa']` | no (pages) | ✅ |
| wuxiaworld | `['novel']` | yes (chapterText, no pages) | ✅ |
| manga18fx | `['manhwa']` | no (pages) | ✅ |

## Plugin Contract — Existing (`plugin-host/src/contract.test.ts`) — `novelupdates` added ✅

| Plugin | Cases | Status |
|---|---|---|
| novelupdates | id, default_explicit=false, content_types (novel), search+chapters exports, no pages | ✅ |
| plugin-index consistency | novelupdates index.json includes novel | ✅ |

## Plugin Behavior — New Plugins (`plugin-host/src/behavior.new-plugins.test.ts`) ✅ ADR 0031

HTML/JSON fixture tests for scraping logic. Each plugin tested with mocked responses.

| Plugin | Cases | Status |
|---|---|---|
| mangafire | search shape + field values, manhwa type, chapters w/ numbers, pages URLs | ✅ |
| asurascans | search shape + slug extraction, status normalization, chapters, pages | ✅ |
| wuxiaworld | search shape + API mapping, chapters returns all from chapterGroups (count, ch1 real slug, ch2+ numeric source_id, volume from group order, empty fallback), chapterText extraction | ✅ |
| manga18fx | search shape + slug extraction, chapters with numbers, pages URLs | ✅ |
| manga18fx | chapters — sidebar/popular chapter links from other series NOT included (contamination regression) | ✅ |
| manga18fx | search URL is `/search?q=` not `/?s=` (WordPress fallback regression) | ✅ |
| manga18fx | pages — lazy-load URLs match any `imgXX.manga18fx.com` CDN subdomain (not hardcoded to `img01`) | ✅ |
| manga18fx | pages — mixed lazy+eager: some imgs have `data-src`, some have `src` only — all CDN URLs returned | ✅ |
| novelupdates | `parseSearchHtml` — id/title/status/cover, multiple results, empty HTML, status mapping | ✅ |

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

Every web test automatically receives `layer=UI` and `tag=Web` via `beforeEach` in `web/src/test-setup.ts`.

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
cd web && npm test
cd web && npm run test:coverage   # with coverage

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
