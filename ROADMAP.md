# *ARRgh Roadmap

Items marked ✅ are shipped. 🔳 = planned. Open an issue to propose or claim one.
Last updated for **v1.2.0** (2026-09-27). Per-release detail: [CHANGELOG.md](CHANGELOG.md).

## **Platform**
✅ Backend rewritten in Rust (axum + sqlx), frontend in Svelte 5 — v1.0.0<br />
✅ Automatic schema migrations on startup (sqlx migrate) — no manual DB steps on upgrade<br />
✅ Docker Compose, Portainer and example Kubernetes deployments<br />
✅ One-command local dev stack (`scripts/dev-up.sh`)<br />
🔳 PostgreSQL support alongside SQLite<br />

## **Sources**
✅ Source plugin system — add sources without recompiling<br />
✅ Plugin Host — all plugins run in a single Node.js container; no per-source ports<br />
✅ Plugin browse + install UI — Settings → Sources → Browse<br />
✅ CloakBrowser sidecar — CF-protected sources use stealth Chromium via CDP<br />
✅ MangaDex, Mangapill (manga) · Toonily, AsuraScans, Manga18fx (manhwa)<br />
✅ NovelFull, NovelFull.net, WuxiaWorld, Royal Road (novels) · nhentai (hentai)<br />
✅ Multi-source chapter pool with priority-ordered download fallback<br />
✅ Plugin call timeout — a hung source can't stall search, sync or downloads — v1.2.0<br />
🔳 One repo per plugin + one-click plugin updates from Settings, with checksum verification and revert to bundled ([#199](https://github.com/t2vi/arrgh/issues/199))<br />
🔳 Manually set an alternate title for source matching (e.g. a novel's other edition) ([#193](https://github.com/t2vi/arrgh/issues/193))<br />

## **Discover / Trending**
✅ Fan-out search across 7 authorities (MangaUpdates, AniList, MangaDex, NovelUpdates, WuxiaWorld, Royal Road, nhentai)<br />
✅ English-original novels via Royal Road — v1.2.0<br />
✅ Live per-source progress, results stream in as each source answers — v1.2.0<br />
✅ Trending lanes: Manga, Manhwa, Manhua, Adult Manhwa<br />
🔳 Exact title matches ranked above partial matches ([#190](https://github.com/t2vi/arrgh/issues/190))<br />
🔳 Trending hentai lane<br />
🔳 Trending novels lane<br />

## **Downloads**
✅ Per-chapter download progress — live percentage bar in Downloads queue and title view<br />
✅ Parallel downloads — `download_workers` (1–10) honoured, live-adjustable — v1.2.0<br />
✅ Scheduled re-sync + auto-download of newly found chapters (global default, per-title override) ([#200](https://github.com/t2vi/arrgh/issues/200))<br />

## **Reader**
✅ Paged and scroll modes for comics (scroll default since v1.1.0)<br />
✅ Novel reader (Markdown prose)<br />
🔳 Novel reader typography controls (font size, line width, serif/sans)<br />
🔳 Keyboard and remote shortcuts in web reader<br />
🔳 Reading statistics (time spent, chapters per week)<br />

## **Library**
✅ Library sort & filter (recently added, title, year; content type, status) — v0.1.7<br />
✅ Sync warnings + re-sync; chapter renumbering handled on re-sync — v1.2.0<br />
🔳 Metadata editing (title, cover, tags)<br />
🔳 CBZ / CBR local import<br />
🔳 Backup and export (library + reading progress)<br />

## **Content Types**
✅ Hentai as a distinct content type, with nhentai as its authority and explicit-only source<br />
🔳 Hentai label separate from manga in UI, library counts<br />
🔳 Dashboard hero stats: manga and hentai counts displayed separately<br />

## **UI / UX**
🔳 Mobile-responsive layout<br />
🔳 Dashboard "My Library" categories shown as pills instead of CSV text<br />

## **Server / Observability**
✅ In-app log viewer with runtime capture level — Settings → Logs<br />
✅ Failure log lines name the request method and path — v1.2.0<br />
🔳 Structured log export (JSON download)<br />

## **Integrations**
🔳 Push notifications for new chapters<br />
🔳 Webhook on new chapter download<br />

## **Code health**
🔳 Constrain status/role/content-type values in the schema instead of free-text strings ([#161](https://github.com/t2vi/arrgh/issues/161), [#162](https://github.com/t2vi/arrgh/issues/162), [#163](https://github.com/t2vi/arrgh/issues/163))<br />
🔳 Integration tests for media routes; Hurl API coverage for remaining route groups ([#165](https://github.com/t2vi/arrgh/issues/165), [#166](https://github.com/t2vi/arrgh/issues/166))<br />
🔳 Scheduled live-snapshot job to catch source layout changes automatically<br />
