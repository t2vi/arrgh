# Test Coverage Plan

Strategy: ADR 0012 — three-layer pyramid (Unit → Integration → E2e), sequential in CI, all reporting to Allure at `/test-reports/`. See `docs/adr/0012-testing-strategy.md`.

**Frameworks**
- Web unit: Vitest + @testing-library/react + allure-vitest
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
| Discover | `useDiscover` | content types, submit, blank guard, reset, navigate, 502 | ✅ |
| Discover | `ContentTypeFilter` | render, onChange | ✅ |
| Home | `useHome` | — | ✅ |
| Home | `Cards` | render variants | ✅ |
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
| Manga Detail | `ChapterRow` | no source_id + not downloaded — HardDrive (unlinked) | ✅ |
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

## Running tests

```bash
# Web
cd web && npm test
cd web && npm run test:coverage   # with coverage

# Server
cd server && cargo test
cd server && cargo nextest run    # via nextest (used in CI)
```
