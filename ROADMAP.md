# *ARRgh Roadmap

Items marked ✅ are shipped. 🔳 = planned. Open an issue to propose or claim one.

## **Sources**
✅ Source plugin system — add sources without recompiling <br />
✅ Plugin Host — all plugins run in a single Node.js container; no per-source ports<br />
✅ Plugin browse + install UI — Settings → Sources → Browse<br />
✅ CloakBrowser sidecar — CF-protected sources use stealth Chromium via CDP; no FlareSolverr<br />
✅ MangaDex (manga / manhwa / manhua / one-shot)<br />
✅ Mangapill (manga)<br />
✅ Toonily (manhwa)<br />
✅ NovelFull (xianxia / wuxia novels)<br />
✅ Multi-source fan-out — parallel search + trending, merged by title<br />
✅ Title metadata cache — covers eagerly downloaded locally, CDN-gating transparent to client<br />
✅ Library cover fix — `serve_cover` and `serve_meta_cover` self-heal stale/missing local paths<br />

## **Downloads**
✅ Per-chapter download progress — live percentage bar in Downloads queue and manga detail view<br />

## **Reader**
✅ Light novel reader (text-based, Markdown)<br />
🔳 Novel reader typography controls (font size, line width, serif/sans)<br />
🔳 Keyboard and remote shortcuts in web reader<br />
🔳 Reading statistics (time spent, chapters per week)<br />

## **Library**
🔳 Library sorting and filtering (by status, content type, author, etc.)<br />
🔳 Metadata editing (title, cover, tags)<br />
🔳 CBZ / CBR local import<br />
🔳 Backup and export (library + reading progress)<br />

## **Discover / Trending**
🔳 Trending hentai lane<br />
🔳 Trending novels lane<br />

## **Content Types**
✅ Hentai as a distinct content type — nhentai replaces E-Hentai as hentai metadata authority in discover fan-out; E-Hentai removed (451 geo-blocked); nhentai results return `content_type: "hentai"` → source matching correctly targets nhentai plugin → chapters load<br />
🔳 Hentai label separate from manga in UI, library counts<br />
🔳 Dashboard hero stats: manga and hentai counts displayed separately<br />

## **UI / UX**
🔳 Mobile-responsive layout<br />
🔳 Dashboard "My Library" categories shown as pills instead of CSV text<br />

## **Server / Observability**
✅ In-memory log ring buffer — `GET /api/logs` streams recent server events to the web UI<br />
✅ Runtime log level control — Settings → Logs → adjust capture level without restart<br />
🔳 Structured log export (JSON download)<br />

## **Infrastructure**
🔳 Proper upgrade migration path — documented scripts and tooling so users can upgrade without manual DB steps<br />
🔳 PostgreSQL support alongside SQLite<br />

## **Integrations**
🔳 Push notifications for new chapters<br />
🔳 Webhook on new chapter download<br />

## **Testing**
✅ Live source snapshot tests — `live-tests/` package; real HTTP calls against all sources; adversarial corpus (slug hyphens, `(Novel)` suffix, special chars); raw response + parsed output captured per source. Run on-demand, not in CI; snapshots drive behavior fixture updates. See ADR 0033.<br />
✅ API live tests — `api-live-tests/` package; Vitest snapshot tests hitting a running server; covers discover (manga/manhwa/novel/hentai corpus), sources (nhentai=hentai assertion), and full library flow (discover→add→sync→download→view). Run on-demand with `API_USER`/`API_PASS`. Requires plugin-host + CloakBrowser for hentai flow.<br />
🔳 Scheduled CI snapshot job — detect source layout changes automatically, open PR with diff when snapshots change (ADR 0033 long-term goal)<br />

## **Known Bugs**
✅ nhentai separated from manga — `content_types` changed to `["hentai"]` in plugin, plugin-index, and external_sources seed; migration updates existing rows; nhentai now only queried for hentai searches<br />
✅ nhentai as hentai authority — E-Hentai replaced by nhentai plugin in discover fan-out (E-Hentai was 451 geo-blocked); nhentai results return `content_type: "hentai"`; CloakBrowser hostname rewrite added to plugin-host `getBrowser()` for Docker-on-host dev setups<br />
✅ WuxiaWorld plugin fixed — migrated to `www.wuxiaworld.com/api` (search) + HTML scraping for chapters/chapterText; live test passing<br />
✅ Removed broken plugins — royalroad (ADR 0024 remnant), boxnovel (domain parked), manhuafast (CF managed challenge); plugin dirs, contract/behavior tests, plugin-index entries, and seed rows all purged; migration deletes existing DB rows<br />
