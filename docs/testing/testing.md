# Test Coverage Plan

Strategy: ADR 0012 — three-layer pyramid (Unit → Integration → E2e), sequential in CI, all reporting to Allure at `/test-reports/`. See `docs/adr/0012-testing-strategy.md`.

**Frameworks**
- Web unit: Vitest + @testing-library/react + `allure-vitest@^2.x` (must stay v2 — v3 incompatible with vitest v2)
- Server unit + integration: cargo nextest + cargo-llvm-cov
- E2e: Playwright + allure-playwright (Docker Compose test stack + Fixture Plugin)

**TDD**: write failing test first, then implement. Red → Green → Refactor.

Legend: ✅ exists · 🔳 planned · ❌ gap (needed, not planned yet)

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
| Library | `MangaCard` | render, remove button | ✅ |
| Discover | `useDiscover` | submit, blank guard, navigate, 502, added tracking | ✅ |
| Home | `useHome` | — | ✅ |
| Home | `Cards` | render variants, title+author below cover, error→emoji | ✅ |
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

## Server — Unit (cargo nextest)

### Auth (`src/auth.rs`) ✅

| Case | Status |
|---|---|
| `create_token` → `validate_token` roundtrip — claims preserved | ✅ |
| Wrong secret rejected | ✅ |
| Member role + allow_explicit=false preserved | ✅ |

### Config (`src/config.rs`) ✅

| Case | Status |
|---|---|
| Defaults when no env vars set | ✅ |
| Unparseable `INDEX_INTERVAL_HOURS` falls back to 6 | ✅ |
| `BIND_ADDR` present when set | ✅ |

### Logging (`src/logging.rs`) ✅

| Case | Status |
|---|---|
| Level roundtrip (all 4 levels) | ✅ |
| `level_from_str` case-insensitive | ✅ |
| Unknown level string → None | ✅ |
| Unknown u8 → "INFO" default | ✅ |
| Ring buffer evicts oldest at capacity | ✅ |

### Source (`src/indexer/source.rs`) ✅

| Case | Status |
|---|---|
| Safe title unchanged | ✅ |
| Unsafe chars (`/\:*?"<>|`) replaced with `_` | ✅ |
| Whitespace trimmed | ✅ |
| Empty string | ✅ |

### Discover (`src/api/discover.rs`) ✅

| Case | Status |
|---|---|
| `normalize_title` lowercases | ✅ |
| `normalize_title` strips punctuation | ✅ |
| `normalize_title` collapses whitespace | ✅ |
| `normalize_title` empty string | ✅ |
| `merge_hits` deduplicates same title | ✅ |
| `merge_hits` preserves insertion order | ✅ |
| `merge_hits` keeps distinct titles separate | ✅ |
| `merge_hits` normalizes before dedup (`One-Piece` == `One Piece`) | ✅ |
| `title_matches` exact | ✅ |
| `title_matches` `(Novel)` suffix vs bare site result (short title, levenshtein threshold) | ✅ |
| `title_matches` rejects unrelated titles | ✅ |
| `title_matches` tolerates small typo | ✅ |
| `strip_search_qualifier` strips `(Novel)` | ✅ |
| `strip_search_qualifier` strips `(Manga)` | ✅ |
| `strip_search_qualifier` returns `None` when no qualifier | ✅ |
| `strip_search_qualifier` mid-string paren not stripped | ✅ |
| `strip_search_qualifier` returns `None` when only qualifier remains | ✅ |
| `search_candidates` stripped form first (avoids wasted CloakBrowser call) | ✅ |
| `search_candidates` no duplicates | ✅ |
| `search_candidates` aliases also stripped | ✅ |
| `known_norms` includes stripped form so short titles match | ✅ |
| `known_norms` deduplicates | ✅ |
| `known_norms` end-to-end: short novel title matches site result | ✅ |
| Source routing: `"hentai"` tag routes to explicit pool | ✅ |
| Source routing: `"adult"` tag does NOT route to explicit pool | ✅ |
| Source routing: mixed `"adult"+"hentai"` → explicit pool | ✅ |
| Source routing: empty tags → non-explicit pool | ✅ |
| Source routing: `"HENTAI"` case-insensitive | ✅ |

### Media helpers (`src/media/mod.rs`) ✅

| Case | Status |
|---|---|
| `is_image` accepts known extensions (jpg, jpeg, PNG, WEBP, avif) | ✅ |
| `is_image` rejects non-image (txt, cbz, no ext, html) | ✅ |
| `strip_jpeg_icc` — non-JPEG passed through unchanged | ✅ |
| `strip_jpeg_icc` — too-short data passed through | ✅ |
| `strip_jpeg_icc` — JPEG without ICC passes through | ✅ |
| `strip_jpeg_icc` — APP2 ICC_PROFILE segment stripped | ✅ |

### ExternalSource (`src/indexer/external.rs`) ✅

| Case | Status |
|---|---|
| `sync_chapters` deduplicates chapters by `(title_id, number)` across two sources | ✅ |
| `sync_chapters` is idempotent — syncing same source twice produces no duplicate rows | ✅ |
| `sync_chapters` ON CONFLICT updates `source_id` when plugin returns a new identifier | ✅ |
| `sync_chapters` returns `Err` on 502 (source temporarily unavailable) | ✅ |
| `sync_chapters` preserves existing chapters when source returns 502 | ✅ |
| `sync_chapters` preserves existing chapters when source returns 502 | ✅ |

### Media API (`src/api/media.rs`) ✅

| Case | Status |
|---|---|
| JPEG magic bytes → `"image/jpeg"` | ✅ |
| PNG magic bytes → `"image/png"` | ✅ |
| WEBP magic bytes → `"image/webp"` | ✅ |
| GIF magic bytes → `"image/gif"` | ✅ |
| Too short → None | ✅ |
| Unknown bytes → None | ✅ |
| `root_domain_referer` — extracts root from subdomain | ✅ |
| `root_domain_referer` — apex domain unchanged | ✅ |
| `root_domain_referer` — invalid URL returns empty | ✅ |
| `root_domain_referer` — scheme preserved | ✅ |

### MangaUpdates client (`src/mangaupdates.rs`) ✅

| Case | Status |
|---|---|
| `releases/search` — numeric `series_id` deserialises | ✅ |
| `releases/search` — string `series_id` deserialises | ✅ |
| `releases/search` — null `series_id` → `None` (no error) | ✅ |
| `releases/search` — null `metadata` field ignored | ✅ |
| `releases/search` — null `series` field ignored | ✅ |
| `releases/search` — missing `metadata` field (omitted key) | ✅ |
| `releases/search` — empty results array | ✅ |
| `SeriesRecord` — numeric `series_id` | ✅ |
| `SeriesRecord` — string `series_id` | ✅ |
| `map_series` — strips HTML tags from description | ✅ |
| `map_series` — content type mapping (Manhwa/Manhua/Novel/Manga/null) | ✅ |
| `map_series` — year as string parses to i64 | ✅ |
| `map_series` — prefers `"Author"` type over first in list | ✅ |
| `map_series` — explicit genre tags normalised (Hentai→`hentai`, Smut→`adult`) | ✅ |
| `strip_html` — removes tags | ✅ |
| `strip_html` — plain text unchanged | ✅ |
| `strip_html` — trims surrounding whitespace | ✅ |

### Downloader (`src/downloader/mod.rs`) ✅

| Case | Status |
|---|---|
| `download_cbz` returns `Err` when source returns 0 pages | ✅ |

---

## Server — Integration (HTTP-level, in-memory SQLite)

| Area | Case | Status |
|---|---|---|
| Auth | No token → 401 | ✅ |
| Auth | Valid token → handler runs | ✅ |
| Auth | Media routes bypass auth | ✅ |
| Settings | `GET /api/settings` returns 200 JSON object | ✅ |
| Settings | `POST /api/settings` persists + returns updated | ✅ |
| Settings | `POST /api/settings` invalid reader_mode → 422 | ✅ |
| Logs | `GET /api/logs` returns empty array on fresh buffer | ✅ |
| Logs | `PATCH /api/logs/level` requires admin (member → 403) | ✅ |
| Logs | `PATCH /api/logs/level` admin → 204 | ✅ |
| Version | `GET /api/version` returns current without latest (check disabled) | ✅ |
| Sources | Add + reload registry | ✅ |
| Multi-source: title schema | `is_local=true` when no `title_sources` rows | ✅ |
| Multi-source: title schema | `is_local=false` when `title_sources` row exists | ✅ |
| Multi-source: chapter schema | `has_sources=false` when no `chapter_sources` rows | ✅ |
| Multi-source: chapter schema | `has_sources=true` when `chapter_sources` row exists | ✅ |
| Multi-source: download guard | `POST /chapters/:id/download` → 404 without `chapter_sources` | ✅ |
| Multi-source: download guard | `POST /chapters/:id/download` → 202 with `chapter_sources` | ✅ |
| Multi-source: sync | `POST /titles/:id/sync` → 404 when no `title_sources` | ✅ |
| Multi-source: sync | `POST /titles/:id/sync` → 202 when `title_sources` exist | ✅ |
| add_title (MU) | Creates titles row with `mangaupdates_id` | ✅ |
| add_title (MU) | Creates `user_titles` subscription for the requesting user | ✅ |
| add_title (MU) | Sets `is_explicit = 1` when tags contain `"adult"` | ✅ |
| add_title (MU) | Deduplicates — same `mangaupdates_id` returns same title | ✅ |
| add_title: qualifier stripping | `"My Title (Novel)"` stored as `"My Title"` | ✅ |
| add_title: qualifier stripping | `"Berserk (Manga)"` stored as `"Berserk"` | ✅ |
| add_title: qualifier stripping | Plain title without qualifier stored unchanged | ✅ |
| add_title: explicit routing | `"adult"` tag → `is_explicit=1` but tags contain no `"hentai"` (routing stays non-explicit) | ✅ |
| Stale sync warning | `"no matching source found"` warning deleted when title has a matched source | ✅ |
| Stale sync warning | Chapter-sync-failure warning preserved — not cleared by no-match cleanup | ✅ |
| Trending | Returns pre-seeded cache results without hitting MangaUpdates network | ✅ |
| Trending | Marks `in_library=true` when series already added to library | ✅ |
| Queue: explicit filter | Member without `allow_explicit` cannot see explicit queue items | ✅ |
| Queue: explicit filter | Member with `allow_explicit` sees explicit queue items | ✅ |
| Queue: explicit filter | Admin sees explicit queue items regardless of `allow_explicit` flag | ✅ |
| Queue: clear_completed | Member → 403 | ✅ |
| Queue: clear_completed | Admin → 204 | ✅ |
| Queue: cancel ownership | Member cannot cancel another user's item → 403 | ✅ |
| Queue: cancel ownership | Member can cancel their own item → 204 | ✅ |
| Queue: cancel ownership | Admin can cancel any item → 204 | ✅ |
| Queue: delete_files | Member remove with `?delete_files=true` returns 204 (param silently ignored) | ✅ |
| ExternalSource | `sync_chapters` deduplicates chapters by `(title_id, number)` across two sources | ✅ |
| Sync log | `GET /api/titles/:id/sync-log` — empty array when no entries | ✅ |
| Sync log | `GET /api/titles/:id/sync-log` — returns entries in ASC order | ✅ |
| Sync log | `GET /api/titles/:id/sync-log` — 404 for title not in user's library | ✅ |
| Cover CDN fallback | `GET /api/media/cover/:id` — 307 to CDN when `cover_url = NULL` | ✅ |
| Cover CDN fallback | `GET /api/media/cover/:id` — 307 to CDN when local file missing | ✅ |
| Cover CDN fallback | `GET /api/media/cover/:id` — 404 when no `title_meta` CDN URL | ✅ |

---

## .NET Server — Unit (xUnit, `server-dotnet-tests/`)

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

---

## .NET Server — Integration (HTTP stack, `server-dotnet-tests/`)

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
| `PATCH /me` → changes password; 422 short password | ✅ |
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
| `DELETE /queue/:id` → cancels in-progress item instead of deleting | ✅ |
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
| `GET /sources` → `has_api_key=true` when API key set | ✅ |
| `GET /sources` → 401 no token | ✅ |
| `POST /sources` → 403 for member | ✅ |
| `POST /sources` → 502 (plugin host not ported yet) | ✅ |
| `PATCH /sources/:id` → toggles enabled | ✅ |
| `PATCH /sources/:id` → 404 nonexistent | ✅ |
| `PATCH /sources/:id` → 403 for member | ✅ |
| `DELETE /sources/:id` → 204 when exists | ✅ |
| `DELETE /sources/:id` → 404 nonexistent | ✅ |
| `DELETE /sources/:id` → 403 for member | ✅ |

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

### Discover (`DiscoverTests.cs`) ✅

| Case | Status |
|---|---|
| `GET /discover` → 401 without token | ✅ |
| `GET /discover` → 502 when MangaUpdates fails | ✅ |
| `GET /discover` → returns mapped results with `in_library=false` | ✅ |
| `GET /discover` → `in_library=true` when title already in library | ✅ |
| `GET /discover/trending` → 401 without token | ✅ |
| `GET /discover/trending` → 502 when MU fails and no cached data | ✅ |
| `GET /discover/trending` → serves stale cache when MU fails | ✅ |
| `POST /discover/add` → 401 without token | ✅ |
| `POST /discover/add` → creates title with qualifier stripped + returns `sync_status=syncing` | ✅ |
| `POST /discover/add` → duplicate MU ID subscribes user and returns existing title | ✅ |
| `POST /discover/add` → explicit tags → `is_explicit=true` | ✅ |

---

## E2e — Playwright (Docker Compose + Fixture Plugin)

See ADR 0023 for full architecture decisions (fixture server, isolation, CI, shared Allure cache).

**Infrastructure**: `docker-compose.test.yml` — replaces plugin-host with standalone Fixture Plugin server at `http://fixture:4001`. No CloakBrowser. Admin seeded via `global-setup.ts` → `POST /api/auth/register`.

**Fixture modes** (`plugins/fixture/`):

| Title | Fixture behaviour |
|---|---|
| "Fixture Manga" | Returns 3 chapters, 3 pages (tiny JPEG at `/image.jpg`) |
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
| Queue chapter → download reaches `done` | `download.spec.ts` | Fixture Manga | ✅ |
| Source 502 on chapter sync → sync warning shown, status `ready` | `download.spec.ts` | Fixture 502 | ✅ |
| Empty pages → queue item shows error state | `download.spec.ts` | Fixture Empty Pages | ✅ |
| Navigate to bad title URL → error state, no crash | `discover.spec.ts` | none | ✅ |
| Discover page loads with search input | `discover.spec.ts` | none | ✅ |

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

Server tests appear under their Rust module paths in the Suites view (from JUnit XML classnames).

---

## Running tests

```bash
# Web
cd web && npm test
cd web && npm run test:coverage   # with coverage

# Rust server
cd server && cargo test
cd server && cargo nextest run    # via nextest (used in CI)

# .NET server — unit first (faster, fail-fast), then integration
cd server-dotnet-tests && dotnet test --filter "Category=Unit"
cd server-dotnet-tests && dotnet test --filter "Category=Integration"
cd server-dotnet-tests && dotnet test              # all at once (local dev only)

# E2e (requires Docker)
docker compose -f docker-compose.test.yml up -d --build
cd e2e && npm ci && npx playwright install chromium --with-deps && npm test
docker compose -f docker-compose.test.yml down -v
```
