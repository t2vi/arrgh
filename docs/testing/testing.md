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
| `merge_hits` deduplicates same title | ✅ |
| `merge_hits` preserves insertion order | ✅ |
| `merge_hits` keeps distinct titles separate | ✅ |
| `merge_hits` normalizes before dedup (`One-Piece` == `One Piece`) | ✅ |

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
| `sync_chapters` deduplicates chapters by `(manga_id, number)` across two sources | ✅ |
| `sync_chapters` is idempotent — syncing same source twice produces no duplicate rows | ✅ |
| `sync_chapters` ON CONFLICT updates `source_id` when plugin returns a new identifier | ✅ |
| `sync_chapters` returns Ok(0) on 502 (source temporarily unavailable) | ✅ |
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
| Sources | Add + reload registry | 🔳 |
| Multi-source: manga schema | `is_local=true` when no `manga_sources` rows | ✅ |
| Multi-source: manga schema | `is_local=false` when `manga_sources` row exists | ✅ |
| Multi-source: chapter schema | `has_sources=false` when no `chapter_sources` rows | ✅ |
| Multi-source: chapter schema | `has_sources=true` when `chapter_sources` row exists | ✅ |
| Multi-source: download guard | `POST /chapters/:id/download` → 404 without `chapter_sources` | ✅ |
| Multi-source: download guard | `POST /chapters/:id/download` → 202 with `chapter_sources` | ✅ |
| Multi-source: sync | `POST /manga/:id/sync` → 404 when no `manga_sources` | ✅ |
| Multi-source: sync | `POST /manga/:id/sync` → 202 when `manga_sources` exist | ✅ |
| add_manga (MU) | Creates manga row with `mangaupdates_id` | ✅ |
| add_manga (MU) | Creates `user_manga` subscription for the requesting user | ✅ |
| add_manga (MU) | Sets `is_explicit = 1` when tags contain `"adult"` | ✅ |
| add_manga (MU) | Deduplicates — same `mangaupdates_id` returns same manga | ✅ |
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

---

## E2e — Playwright (Docker Compose + Fixture Plugin)

Batch 1:

| Flow | Steps | Status |
|---|---|---|
| Auth | Register → login → logout → login again | 🔳 |
| Library | Login → add manga via Discover → appears in Library | 🔳 |
| Download | Add manga → queue chapter → status reaches `done` | 🔳 |
| Discover | Search via Fixture Plugin → results shown → add to library | 🔳 |

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

# Server
cd server && cargo test
cd server && cargo nextest run    # via nextest (used in CI)
```
