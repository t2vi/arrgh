# Test Coverage Plan

Framework: **Vitest + @testing-library/react** (web), **cargo test** (server, not yet wired).

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
| Queue | `QueueRow` | **progress bar shown when downloading + pages_total > 0** | ✅ |
| Queue | `QueueRow` | **progress bar hidden when pages_total = 0** | ✅ |
| Queue | `QueueRow` | **percentage text matches pages_downloaded/pages_total** | ✅ |
| Settings | `LogsSection` | renders level selector and log table | 🔳 |
| Settings | `LogsSection` | filter selector hides entries below selected level | 🔳 |
| Settings | `LogsSection` | setLogLevel called when capture level changes | 🔳 |
| Manga Detail | `ChapterRow` | downloaded state — shows BookOpen, no download icon | 🔳 |
| Manga Detail | `ChapterRow` | pending state — shows Queued button, cancel calls onCancelDownload | 🔳 |
| Manga Detail | `ChapterRow` | active state (downloading) — shows spinner, no remove btn | 🔳 |
| Manga Detail | `ChapterRow` | active + pages_total > 0 — shows progress bar + percentage | 🔳 |
| Manga Detail | `ChapterRow` | active + pages_total = 0 — no progress bar | 🔳 |
| Manga Detail | `ChapterRow` | error state — shows AlertCircle | 🔳 |
| Manga Detail | `ChapterRow` | completed progress — read bar at 100%, opacity-50 | 🔳 |
| Manga Detail | `ChapterRow` | no source_id + not downloaded — shows HardDrive (unlinked) | 🔳 |
| Settings | `useSettings` | browse modal open/close | 🔳 |
| Settings | `useSettings` | install plugin success → sources refetch | 🔳 |
| Settings | `useSettings` | install plugin error → error message shown | 🔳 |

---

## Server — Unit (cargo test)

Not yet written. No test infra scaffolded. Planned cases below.

### Auth (`src/auth.rs`)

| Case | What to assert |
|---|---|
| `create_token` → `validate_token` round-trip | claims.sub == user_id, claims.role preserved |
| `validate_token` with wrong secret | returns Err |
| `validate_token` with expired token | returns Err |

### Config (`src/config.rs`)

| Case | What to assert |
|---|---|
| All env vars set | all fields populated correctly |
| No env vars | defaults applied (`sqlite://arrgh.db`, `./downloads`, interval=6, etc.) |
| `INDEX_INTERVAL_HOURS` non-numeric | falls back to default (6) |

### API error handling (`src/api/mod.rs`)

| Case | What to assert |
|---|---|
| `AppError::from(anyhow::anyhow!(...))` | `into_response()` returns 500 |

### Sources registry (`src/api/sources.rs`)

| Case | What to assert |
|---|---|
| Concurrent `reload_registry` calls | second waits for first; final state is consistent (requires in-memory SQLite) |

### Media (`src/api/media.rs`)

| Case | What to assert |
|---|---|
| `image_content_type` — JPEG magic bytes | returns `"image/jpeg"` |
| `image_content_type` — PNG magic bytes | returns `"image/png"` |
| `image_content_type` — WEBP magic bytes | returns `"image/webp"` |
| `image_content_type` — too short | returns None |
| `root_domain_referer` — full URL | returns scheme + root domain only |
| `root_domain_referer` — invalid URL | returns empty string |

### Downloader (`src/downloader/mod.rs`)

| Case | What to assert |
|---|---|
| `pages_downloaded` increments per page | after N pages, value == N |
| Transaction atomicity: chapter downloaded=1 + queue done in one tx | partial write leaves neither updated |

---

## Server — Integration (cargo test, in-memory SQLite)

Requires: `sqlx::SqlitePool` with `:memory:` + migrations applied.

| Area | Case |
|---|---|
| Auth middleware | request without token → 401 |
| Auth middleware | request with valid token → handler runs |
| Auth middleware | media routes bypass auth → 200 without token |
| Plugin install | DB insert + in-memory registry updated |
| Plugin delete | DB delete + in-memory registry updated |
| Source add/patch/delete | registry reflects DB state after each op |

---

## E2E (not planned)

No E2E framework configured. Playwright would be the natural choice given it's already a dependency of plugin-host. Not prioritised until core unit/integration gaps are closed.

---

## Running tests

```bash
# Web
cd web && npm test

# Server (once tests exist)
cd server && cargo test
cd server && cargo test -- --nocapture   # with output
```
