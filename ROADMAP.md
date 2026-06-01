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
🔳 Hentai as a distinct content type — separate label from manga in UI, library counts, and source routing<br />
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
🔳 Scheduled CI snapshot job — detect source layout changes automatically, open PR with diff when snapshots change (ADR 0033 long-term goal)<br />

## **Known Bugs**
🔳 Regular (non-explicit) manga being matched to nhentai — explicit-only sources must not match non-explicit titles<br />
🔳 WuxiaWorld plugin broken — `api2.wuxiaworld.com` returns 404; plugin needs new API endpoint or replacement source<br />
🔳 Royal Road not fully removed — source code remains at `plugins/royalroad/`, still referenced in `plugin-host/src/contract.test.ts`; should be purged entirely (ADR 0024 removed it from bundled set but left files behind)<br />
🔳 BoxNovel plugin broken — `boxnovel.com` is domain-parked (redirects to `router.parklogic.com`); plugin needs replacement source or removal<br />
🔳 ManhuaFast blocked by CF managed challenge — `manhuafast.net` uses Cloudflare managed/Turnstile challenge; CloakBrowser cannot bypass it (JS challenge only); all plugin calls return empty results until the CF tier is downgraded or a bypass is found<br />
