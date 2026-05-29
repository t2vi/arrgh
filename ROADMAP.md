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

## **Library**<br
🔳 Metadata editing (title, cover, tags)<br />
🔳 CBZ / CBR local import<br />
🔳 Backup and export (library + reading progress)<br />

## **Server / Observability**
✅ In-memory log ring buffer — `GET /api/logs` streams recent server events to the web UI<br />
✅ Runtime log level control — Settings → Logs → adjust capture level without restart<br />
🔳 Structured log export (JSON download)<br />

## **Integrations**
🔳 Push notifications for new chapters<br />
🔳 Webhook on new chapter download<br />

**Infrastructure**
🔳 PostgreSQL support alongside SQLite
