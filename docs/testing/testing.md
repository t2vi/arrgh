# Test Coverage Plan

Strategy: four-layer pyramid (Unit → Integration → API → E2e), sequential in CI, all reporting to Allure at `/test-reports/`. See `docs/adr/0012-testing-strategy.md`.

**Frameworks**
- Web unit: Vitest + @testing-library/react + `allure-vitest@^2.x` (must stay v2 — v3 incompatible with vitest v2)
- Server unit + integration: .NET xUnit (`Category=Unit` / `Category=Integration`)
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
| Library | `useLibrary` | fetch, totalPages, remove, removingId, syncing poll | ✅ |
| Library | `MangaCard` | render, remove button, is_explicit=true→18+ pill shown, is_explicit=false→no 18+ pill | ✅ |
| Discover | `useDiscover` | submit, blank guard, navigate, added tracking, source field, addingId lifecycle, addError, contentTypeFilter, filteredData, availableTypes (6 TDD ⬜) | 🟡 |
| Discover | `SearchRow` | render, is_explicit=true→18+ badge shown, is_explicit=false→no 18+ badge, tag-based inference blocked, loading state, In Library, cover/skeleton | ✅ |
| Discover | `ContentTypeFilter` | render, hentai pill, novel pill, onChange (2 TDD ⬜) | 🟡 |
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

## .NET Server — Downloader Unit (`DownloaderTests.cs`)

| Case | Status |
|---|---|
| Queue item status set to `"downloading"` (not `"in_progress"`) when tick claims it | ✅ |
| Successful download → queue item `status="done"`, chapter `downloaded=true` | ✅ |
| Source returns 502 on pages endpoint → queue item `status="error"` | ✅ |
| Source returns 400 on single image → queue item `status="error"` with URL in message | ✅ |
| Empty pages list → queue item `status="error"` | ✅ |
| Text chapter → `.md` file written, queue item `status="done"` | ✅ |
| Text chapter source 502 → queue item `status="error"` with URL in message | ✅ |
| User-Agent header sent on all HTTP requests | ✅ |
| Multiple sources — first fails, second succeeds → `status="done"` | ✅ |
| No chapter_sources → queue item `status="error"` ("no chapter sources") | ✅ |

---


## .NET Server — Unit (xUnit, `server-tests/`)

Framework: xUnit + `WebApplicationFactory` (integration) / plain xUnit (unit). Run with `dotnet test --filter "Category=Unit"`.

### Auth tokens (`AuthTokenTests.cs`) ✅

| Case | Status |
|---|---|
| Admin claims preserved through `CreateToken` → `ValidateToken` roundtrip | ✅ |
| Member role + `allow_explicit=false` preserved through roundtrip | ✅ |
| Wrong secret → validation rejected | ✅ |
| Token expires in 30 days | ✅ |
| Different users produce different tokens | ✅ |
| Same inputs produce different tokens (timestamp-based) | ✅ |

### UpdateCache (`UpdateCacheTests.cs`) ✅

| Case | Status |
|---|---|
| `GetIfNewer` empty cache → both fields null | ✅ |
| `GetIfNewer` same version as current → both null (suppresses false "update available") | ✅ |
| `GetIfNewer` newer version → returns version + URL | ✅ |
| `Clear` after set → both null | ✅ |

### LogService (`LogServiceTests.cs`) ✅

| Case | Status |
|---|---|
| `ParseLevel` valid levels (trace/debug/info/warn/error, case-insensitive) | ✅ |
| `ParseLevel` unknown level → null | ✅ |
| `LevelToString` all four levels produce expected strings | ✅ |
| `SetLevel` valid level → true + updates `CurrentLevel` | ✅ |
| `SetLevel` invalid level → false | ✅ |
| `SetLevel` normalises to uppercase | ✅ |
| `GetRecent` empty buffer → empty list | ✅ |
| `GetRecent` returns last N entries | ✅ |
| `Append` evicts oldest entry when capacity exceeded | ✅ |

### MigrationBootstrap (`MigrationBootstrapTests.cs`) ✅

| Case | Status |
|---|---|
| Pre-migration DB (tables exist, no `__EFMigrationsHistory`) → `Bootstrap` creates history → `Migrate()` succeeds | ✅ |
| Fresh DB (no tables) → `Bootstrap` is no-op → `Migrate()` handles it normally | ✅ |
| Already-migrated DB → `Bootstrap` is idempotent → `Migrate()` is no-op | ✅ |

### PatchTitleBody (`PatchTitleBodyTests.cs`) ✅

| Case | Status |
|---|---|
| `auto_download: true` / `false` parsed | ✅ |
| `auto_download` absent → null (no-op) | ✅ |
| `reader_mode` string → `HasValue=true` | ✅ |
| `reader_mode` absent → null | ✅ |
| `reader_mode: null` JSON null — known limitation documented (indistinguishable from absent) | ✅ |
| `is_explicit: true` parsed | ✅ |
| `content_type: "manga"` parsed | ✅ |
| `content_type` absent → null | ✅ |
| Multiple fields all parsed together | ✅ |
| Empty object → all fields null | ✅ |

### Queue logic (`QueueLogicTests.cs`) ✅

| Case | Status |
|---|---|
| `IsAllowedExplicit` — `allow_explicit=true` → true | ✅ |
| `IsAllowedExplicit` — member without flag → false | ✅ |
| `IsAllowedExplicit` — admin with flag=false → true | ✅ |
| `IsAllowedExplicit` — admin with flag=true → true | ✅ |
| `IsAllowedExplicit` — null role + flag=false → false | ✅ |
| `IsAllowedExplicit` — null role + flag=true → true | ✅ |

### Settings logic (`SettingsLogicTests.cs`) ✅

| Case | Status |
|---|---|
| `ParseLong` valid string, null, invalid string | ✅ |
| `ParseBool` "true", "false", null, other string | ✅ |
| `ClampTrending` below 1 → 1; above 50 → 50; within range passes; boundary values | ✅ |
| `ValidReaderMode` "paged" / "scroll" valid; other / empty invalid | ✅ |

### Media helpers (`MediaLogicTests.cs`) ✅

| Case | Status |
|---|---|
| `DetectContentType` — JPEG / PNG / WebP / GIF / AVIF magic bytes | ✅ |
| `DetectContentType` — too short / empty / unknown bytes → null | ✅ |
| `StripJpegIcc` — non-JPEG passed through unchanged | ✅ |
| `StripJpegIcc` — too-short data passed through unchanged | ✅ |
| `StripJpegIcc` — JPEG without ICC passes through | ✅ |
| `StripJpegIcc` — APP2 ICC_PROFILE segment stripped | ✅ |
| `IsImage` — known extensions (jpg, jpeg, PNG, webp, avif) → true | ✅ |
| `IsImage` — non-image (txt, cbz, no-ext, html) → false | ✅ |
| `RootDomainReferer` — subdomain extracts root; apex unchanged; invalid → empty; scheme preserved; empty → empty | ✅ |
| `NormalizeTitle` — replaces non-alphanumeric with spaces; collapses spaces; lowercase; strips punctuation | ✅ |
| `GetChapterPage` — missing path → null; directory reads correct file; directory out of range → null | ✅ |
| `GetChapterPage` — CBZ extracts correct entry; CBZ out of range → null | ✅ |
| `PageCacheService` — miss on empty; hit after set | ✅ |

### Discover helpers (`DiscoverLogicTests.cs`) ✅

| Case | Status |
|---|---|
| `TitleMatches` — exact; both empty; small typo; novel-suffix vs bare site result | ✅ |
| `TitleMatches` — unrelated titles → false | ✅ |
| `Levenshtein` — same string = 0; empty vs non-empty = length; one substitution | ✅ |
| `StripSearchQualifier` — strips `(Novel)` / `(Manga)`; no suffix → null; mid-string paren → null; only paren → null; long suffix → null | ✅ |
| `SearchCandidates` — stripped form first; no duplicates; aliases also stripped | ✅ |
| `KnownNorms` — includes stripped variant; no duplicates; end-to-end novel title matches site result | ✅ |
| `IsHentaiTag` — "hentai" → true; "adult" alone → false; case-insensitive; null/empty → false | ✅ |
| `MangaUpdatesService.MapContentType` — Manhwa/manhua/Novel/Light Novel/Web Novel/Manga/null | ✅ |
| `MangaUpdatesService.StripHtml` — removes tags; plain text unchanged; trims whitespace | ✅ |
| `MangaUpdatesService.ParseFlexULong` — number; string; null → null | ✅ |
| `MangaUpdatesService.MapSeries` — full record (SeriesId, Title, Description, CoverUrl, ContentType, Status, Year, Author, Tags) | ✅ |
| `MangaUpdatesService.MapSeries` — string `series_id` parsed to ulong | ✅ |
| `NovelUpdatesService.ParseHtml` — single result extracts title, slug, status, cover | ✅ |
| `NovelUpdatesService.ParseHtml` — empty HTML → empty list | ✅ |
| `NovelUpdatesService.ParseHtml` — multiple results parsed | ✅ |
| `NovelUpdatesService.ParseHtml` — Ongoing status maps correctly | ✅ |

---

## .NET Server — Integration (HTTP stack, `server-tests/`)

`WebApplicationFactory` + isolated file-based SQLite per test. Run with `dotnet test --filter "Category=Integration"`.

### Auth (`AuthTests.cs`) ✅

| Case | Status |
|---|---|
| `GET /status` → `needs_setup=true` when no users | ✅ |
| `GET /status` → `needs_setup=false` after first register | ✅ |
| `POST /register` → creates admin + returns token | ✅ |
| `POST /register` → 403 when users already exist | ✅ |
| `POST /register` → 422 short password / empty username | ✅ |
| `POST /login` → token for valid credentials | ✅ |
| `POST /login` → 401 wrong password / unknown user | ✅ |
| `GET /me` → current user | ✅ |
| `GET /me` → 401 no token | ✅ |
| `GET /users` → 403 for member | ✅ |
| `GET /users` → returns all users for admin | ✅ |
| `POST /users` → 201 valid member; 403 for member; 409 duplicate username | ✅ |
| `PATCH /me` → changes password or allow_explicit (both optional); returns 200 + updated user; 422 short password | ✅ |
| `PATCH /users/:id` → updates role / allow_explicit; 422 invalid role; 404 nonexistent | ✅ |
| `DELETE /users/:id` → 204 success; 404 nonexistent; 403 cannot delete self; 403 for member | ✅ |

### Titles (`TitlesTests.cs`) ✅

| Case | Status |
|---|---|
| `GET /titles` → empty page for empty library | ✅ |
| `GET /titles` → returns only owned titles | ✅ |
| `GET /titles` → excludes other users' library | ✅ |
| `GET /titles` → hides explicit from non-explicit user | ✅ |
| `GET /titles` → shows explicit to explicit user | ✅ |
| `GET /titles` → search filters by title name | ✅ |
| `GET /titles` → 401 no token | ✅ |
| `GET /titles` → pagination limit and page offset | ✅ |
| `GET /titles` → multi-user each sees only own library | ✅ |
| `GET /titles/:id` → returns owned title with chapter stats | ✅ |
| `GET /titles/:id` → 404 not owned / nonexistent | ✅ |
| `GET /titles/:id` → `chapters_read` isolated per user | ✅ |
| `GET /titles/:id` → `is_local=true` when no title_sources; `is_local=false` when sources exist | ✅ |
| `GET /titles/:id` → `has_sync_warnings=true` when warning exists | ✅ |
| `GET /titles/new-releases` → returns new chapters for owned titles only | ✅ |
| `GET /titles/new-releases` → excludes explicit from non-explicit user | ✅ |
| `DELETE /titles/:id` → 204; does not delete when other user still has it; deletes when last user | ✅ |
| `DELETE /titles/:id` → 404 not owned | ✅ |
| `PATCH /titles/:id` → updates auto_download; sets reader mode (paged/scroll) | ✅ |
| `PATCH /titles/:id` → 403 is_explicit for member; 422 invalid reader mode / content type | ✅ |
| `PATCH /titles/:id` → admin can set is_explicit | ✅ |
| `PATCH /titles/:id` → 404 not owned | ✅ |
| `GET /titles/:id/sync-log` → returns entries in ASC order; 404 not owned | ✅ |
| `POST /titles/:id/sync` → 202 when source links exist; 404 no source links; 404 not owned | ✅ |
| `POST /titles/:id/sync` → sync log contains `"Synced N chapter(s) from {source}"` | ✅ |
| `POST /titles/:id/sync` → sync log contains `"Synced 0 chapter(s)"` when plugin returns empty | ✅ |

### Chapters (`ChaptersTests.cs`) ✅

| Case | Status |
|---|---|
| `GET /chapters/:titleId` → ordered by number | ✅ |
| `GET /chapters/:titleId` → empty when no chapters | ✅ |
| `GET /chapters/:titleId` → `has_sources=true` when chapter_source exists | ✅ |
| `GET /chapters/:titleId` → `has_sources=false` when no chapter_source | ✅ |
| `GET /chapters/:titleId` → hides explicit title from non-allowed user | ✅ |
| `GET /chapters/:titleId` → shows explicit title to allowed user | ✅ |
| `GET /chapters/:titleId` → 401 no token | ✅ |
| `GET /chapters/:chapterId` → returns chapter | ✅ |
| `GET /chapters/:chapterId` → 404 nonexistent / explicit hidden from user | ✅ |
| `GET /chapters/:chapterId/text` → 400 when not text format | ✅ |
| `GET /chapters/:chapterId/text` → 404 when not downloaded | ✅ |

### Progress (`ProgressTests.cs`) ✅

| Case | Status |
|---|---|
| `GET /progress/:titleId` → returns progress for user | ✅ |
| `GET /progress/:titleId` → empty when no progress | ✅ |
| `GET /progress/:titleId` → isolated per user | ✅ |
| `GET /progress/:chapterId/chapter` → returns progress | ✅ |
| `GET /progress/:chapterId/chapter` → 404 when no progress | ✅ |
| `GET /progress/:chapterId/chapter` → 401 no token | ✅ |
| `PUT /progress/:chapterId` → creates when not exists | ✅ |
| `PUT /progress/:chapterId` → updates when already exists | ✅ |
| `PUT /progress/:chapterId` → isolated per user | ✅ |
| `GET /progress/continue-reading` → returns titles with unread chapters | ✅ |
| `GET /progress/continue-reading` → empty when nothing started | ✅ |
| `GET /progress/continue-reading` → empty when all chapters read | ✅ |

### Queue (`QueueTests.cs`) ✅

| Case | Status |
|---|---|
| `GET /queue` → items ordered by created_at DESC | ✅ |
| `GET /queue` → empty when no items | ✅ |
| `GET /queue` → hides explicit items from non-explicit member | ✅ |
| `GET /queue` → shows explicit items to admin | ✅ |
| `GET /queue` → 401 no token | ✅ |
| `GET /queue/:titleId` → items for title ordered by chapter number | ✅ |
| `GET /queue/:titleId` → empty when no items for title | ✅ |
| `DELETE /queue/completed` → deletes done + cancelled + error items | ✅ |
| `DELETE /queue/completed` → 403 for member | ✅ |
| `DELETE /queue/:id` → 204 deletes pending item | ✅ |
| `DELETE /queue/:id` → cancels `"downloading"` item instead of deleting (renamed from "in_progress") | ✅ |
| Seeded mangadex `ContentTypes` does NOT include `"manhwa"` (dedicated sources only) | ✅ |
| Seeded toonily/asurascans have `"manhwa"` in `ContentTypes` | ✅ |
| Seeded mangafire has `"manhwa"` in `ContentTypes` | ✅ |
| Seeded manga18fx has `"manhwa"` content type, `default_explicit=true`, enabled | ✅ |
| `DELETE /queue/:id` → 404 nonexistent | ✅ |

### Settings (`SettingsTests.cs`) ✅

| Case | Status |
|---|---|
| `GET /settings` → defaults when nothing saved | ✅ |
| `GET /settings` → no auth required | ✅ |
| `POST /settings` → updates and returns new values | ✅ |
| `POST /settings` → partial update changes only specified fields | ✅ |
| `POST /settings` → idempotent (overwrites same key) | ✅ |
| `POST /settings` → 422 invalid reader_mode | ✅ |
| `POST /settings` → clamps `trending_per_source` to [1, 50] | ✅ |
| `POST /settings` → ignores empty download_dir | ✅ |
| `POST /settings` → trims download_dir whitespace | ✅ |
| `POST /settings` → no auth required | ✅ |

### Sources (`SourcesTests.cs`) ✅

| Case | Status |
|---|---|
| `GET /sources` → empty when none | ✅ |
| `GET /sources` → returns sources with content_types array | ✅ |
| `GET /sources` → returns `priority` field | ✅ |
| `GET /sources` → `has_api_key=true` when API key set | ✅ |
| `GET /sources` → 401 no token | ✅ |
| `POST /sources` → 403 for member | ✅ |
| `POST /sources` → 502 (plugin host not ported yet) | ✅ |
| `PATCH /sources/:id` → toggles enabled | ✅ |
| `PATCH /sources/:id` → updates priority | ✅ |
| `PATCH /sources/:id` → 404 nonexistent | ✅ |
| `PATCH /sources/:id` → 403 for member | ✅ |
| `DELETE /sources/:id` → 204 when exists | ✅ |
| `DELETE /sources/:id` → 404 nonexistent | ✅ |
| `DELETE /sources/:id` → 403 for member | ✅ |
| Seeded asurascans priority is last (highest number) among all manhwa sources | ✅ |

### Plugins (`PluginsTests.cs`) ✅

| Case | Status |
|---|---|
| `GET /plugins/index` → returns entries from index | ✅ |
| `GET /plugins/index` → 401 without token | ✅ |
| `GET /plugins/index` → member can access | ✅ |
| `POST /plugins/install` → 401 without token; 403 for member | ✅ |
| `POST /plugins/install` → 404 unknown plugin | ✅ |
| `POST /plugins/install` → 422 no download URL | ✅ |
| `POST /plugins/install` → 409 when already installed | ✅ |
| `POST /plugins/install` → 502 when plugin-host fails | ✅ |
| `POST /plugins/install` → 201 on success | ✅ |
| `DELETE /plugins/:id` → 401 without token; 403 for member | ✅ |
| `DELETE /plugins/:id` → 404 unknown | ✅ |
| `DELETE /plugins/:id` → 403 non-community source | ✅ |
| `DELETE /plugins/:id` → 204 removes source | ✅ |
| `FetchIndex` → reads file:// URL | ✅ |
| `FetchIndex` → missing file → null | ✅ |

### Logs (`LogsTests.cs`) ✅

| Case | Status |
|---|---|
| `GET /logs` → empty array on fresh buffer | ✅ |
| `GET /logs` → 401 without token | ✅ |
| `GET /logs` → member can access | ✅ |
| `GET /logs/level` → returns default INFO level | ✅ |
| `GET /logs/level` → 401 without token | ✅ |
| `PATCH /logs/level` → admin updates level | ✅ |
| `PATCH /logs/level` → level persists across requests | ✅ |
| `PATCH /logs/level` → 403 for member | ✅ |
| `PATCH /logs/level` → 401 without token | ✅ |
| `PATCH /logs/level` → 422 invalid level | ✅ |

### Version (`VersionTests.cs`) ✅

| Case | Status |
|---|---|
| `GET /version` → returns current version | ✅ |
| `GET /version` → no auth required | ✅ |
| `GET /version` → no update available → latest + url are null | ✅ |
| `GET /version` → update available → returns latest version + URL | ✅ |

### Discover (`DiscoverTests.cs`) 🟡

| Case | Status |
|---|---|
| `GET /discover` → 401 without token | ✅ |
| `GET /discover` → 502 when MangaUpdates fails | ✅ |
| `GET /discover` → returns mapped results with `in_library=false` | ✅ |
| `GET /discover` → `in_library=true` when title already in library | ✅ |
| `GET /discover` → MU result with "Adult" genre → `is_explicit=true` in response | ✅ |
| `GET /discover` → MU result with no adult genres → `is_explicit=false` in response | ✅ |
| `GET /discover/trending` → 401 without token | ✅ |
| `GET /discover/trending` → 502 when MU fails and no cached data | ✅ |
| `GET /discover/trending` → serves stale cache when MU fails | ✅ |
| `POST /discover/add` → 401 without token | ✅ |
| `POST /discover/add` → creates title with qualifier stripped + returns `sync_status=syncing` | ✅ |
| `POST /discover/add` → duplicate MU ID subscribes user and returns existing title | ✅ |
| `POST /discover/add` → explicit tags → `is_explicit=true` | ✅ |
| `POST /discover/add` → `is_explicit=true` field stores `IsExplicit=true` for manhwa (no hentai tags) | ✅ |
| `POST /discover/add` → `is_explicit=false` does not suppress hentai `content_type` detection | ✅ |

### Discover Fan-Out — Integration (`DiscoverFanOutTests.cs`) ✅ ADR 0031

`FanOutDiscoverFactory` routes HTTP by hostname to all 5 metadata authority fakes.

| Case | Status |
|---|---|
| `GET /discover` → MU manga result has `source="mangaupdates"` | ✅ |
| `GET /discover` → AniList manhwa result has `source="anilist"` | ✅ |
| `GET /discover` → MangaDex manhua result has `source="mangadex"` | ✅ |
| `GET /discover` → NovelUpdates novel result has `source="novelupdates"` | ✅ |
| `GET /discover` → E-Hentai result excluded for non-explicit user | ✅ |
| `GET /discover` → E-Hentai result included for explicit user | ✅ |
| `GET /discover` → dedup: AniList wins for manhwa when MU also returns it | ✅ |
| `GET /discover` → result order: MU before AniList | ✅ |
| `GET /discover` → partial failure (AniList 500) → 200 with other results | ✅ |
| `GET /discover` → all authorities fail → 502 | ✅ |
| `GET /discover` → `in_library=true` by normalized title + content_type (no `mangaupdates_id`) | ✅ |
| `POST /discover/add` → `source="anilist"` stores `metadata_source="anilist"` | ✅ |
| `POST /discover/add` → `mangaupdates_id` only (backward compat) → stores `metadata_source="mangaupdates"` | ✅ |
| `POST /discover/add` → same `source+source_id` twice → deduplicated to one title row | ✅ |
| `GET /discover` → MU novel result excluded even when NovelUpdates returns nothing | ✅ |
| `GET /discover` → MU manhwa result excluded (MU is manga-authority only; ADR 0031) | ✅ |
| `GET /discover` → WuxiaWorld novel result appears with `source="wuxiaworld"` | ✅ |
| `GET /discover` → WuxiaWorld deduped: NovelUpdates wins when both return same novel | ✅ |
| `GET /discover` → WuxiaWorld novels appear even when NovelUpdates fails (CF blocked) | ✅ |
| `POST /discover/add` → novel (`source="novelupdates"`) → sync log does NOT say "Fetching metadata from MangaUpdates" | ✅ |
| `POST /discover/add` → manga (`source="mangaupdates"`) → sync log says "Fetching metadata from MangaUpdates" | ✅ |
| `POST /discover/add` → manga with matching `external_sources` → creates `title_sources` rows | ✅ |
| `POST /discover/add` → no matching `external_sources` → no `title_sources` rows | ✅ |
| `POST /discover/add` → `source="anilist"` → AniList synonyms stored as `TitleAliases` + sync log contains "synonym" | ✅ |
| `POST /discover/add` → `source="anilist"`, empty synonyms → sync reaches "ready", zero aliases stored | ✅ |
| `MatchSourcesAsync` → plugin returns hyphen-variant title ("Soeun" for "So-Eun") → still links source via fuzzy match | ✅ |
| `MatchSourcesAsync` → plugin returns alias-matching title ("Everything Is Agreed" for alias "Everything Is Agreed Upon") → links source | ✅ |
| `MatchSourcesAsync` → plugin returns completely unrelated title → warning logged, no source link created | ✅ |

### Discover Fan-Out — Unit (`DiscoverFanOutLogicTests.cs`) ✅ ADR 0031

| Case | Status |
|---|---|
| `DesignatedAuthority("manga")` → `"mangaupdates"` | ✅ |
| `DesignatedAuthority("manhwa")` → `"anilist"` | ✅ |
| `DesignatedAuthority("manhua")` → `"mangadex"` | ✅ |
| `DesignatedAuthority("novel")` → `"novelupdates"` | ✅ |
| `DesignatedAuthority("hentai")` → `"ehentai"` | ✅ |
| `DesignatedAuthority("unknown")` → `"mangaupdates"` (fallback) | ✅ |
| `Deduplicate` no conflict → returns all | ✅ |
| `Deduplicate` AniList wins for manhwa | ✅ |
| `Deduplicate` MangaDex wins for manhua | ✅ |
| `Deduplicate` same title, different content_type → not deduped | ✅ |
| `Deduplicate` normalized title comparison (extra whitespace) | ✅ |
| `AuthorityOrder` MU before AniList | ✅ |
| `AuthorityOrder` AniList before MangaDex | ✅ |
| `AuthorityOrder` MangaDex before NovelUpdates | ✅ |
| `AuthorityOrder` NovelUpdates before WuxiaWorld | ✅ |
| `AuthorityOrder` WuxiaWorld before E-Hentai | ✅ |
| `MergeFanOut` ordered by authority | ✅ |
| `MergeFanOut` deduplicates before sorting | ✅ |
| `FilterMuScope` excludes novel results | ✅ |
| `FilterMuScope` excludes manhwa results | ✅ |
| `FilterMuScope` excludes manhua results | ✅ |
| `FilterMuScope` excludes hentai results | ✅ |
| `FilterMuScope` keeps manga results | ✅ |
| `FilterMuScope` keeps one-shot results | ✅ |
| `FilterMuScope` mixed input → only manga/one-shot survive | ✅ |

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
| manhuafast | `['manhua']` | no (pages) | ✅ |
| wuxiaworld | `['novel']` | yes (chapterText, no pages) | ✅ |
| boxnovel | `['novel']` | yes (chapterText, no pages) | ✅ |
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
| manhuafast | search shape + slug, status normalization, cover_url, chapters, pages | ✅ |
| wuxiaworld | search shape + API mapping, chapters + source_id, chapterText extraction | ✅ |
| boxnovel | search shape + slug + author, chapters + numbers, chapterText extraction | ✅ |
| manga18fx | search shape + slug extraction, chapters with numbers, pages URLs | ✅ |
| manga18fx | chapters — sidebar/popular chapter links from other series NOT included (contamination regression) | ✅ |
| manga18fx | search URL is `/search?q=` not `/?s=` (WordPress fallback regression) | ✅ |
| manga18fx | pages — lazy-load: `data-src` extracted when `src` is a placeholder GIF | ✅ |
| manga18fx | pages — lazy-load URLs start with `https://img01.manga18fx.com/uploads/` | ✅ |
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

**Note**: Discover endpoints excluded from API layer — calls real external APIs. Covered by .NET integration tests with mocked HTTP.

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

Server tests (xUnit) appear under their class paths in the Suites view.

---

## Running tests

```bash
# .NET server — unit first (faster, fail-fast), then integration
cd server-tests && dotnet test --filter "Category=Unit"
cd server-tests && dotnet test --filter "Category=Integration"
cd server-tests && dotnet test              # all at once (local dev only)

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
