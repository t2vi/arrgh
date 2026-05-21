# *ARRgh Roadmap

Items marked ✅ are shipped. 🔳 = planned. Open an issue to propose or claim one.

**Sources**
✅ Source plugin system — add sources without recompiling
✅ Plugin Host — all plugins run in a single Node.js container; no per-source ports
✅ Plugin browse + install UI — Settings → Sources → Browse
✅ CloakBrowser sidecar — CF-protected sources use stealth Chromium via CDP; no FlareSolverr
✅ MangaDex (manga / manhwa / manhua / one-shot)
✅ Mangapill (manga)
✅ Toonily (manhwa)
✅ Comick (manga / manhwa / manhua) — migrated to `api.comick.dev`
✅ Royal Road (web fiction / novels)
✅ NovelFull (xianxia / wuxia novels)
✅ Multi-source fan-out — parallel search + trending, merged by title
✅ Title metadata cache — covers eagerly downloaded locally, CDN-gating transparent to client
✅ Library cover fix — `serve_cover` and `serve_meta_cover` self-heal stale/missing local paths

**Downloads**
✅ Per-chapter download progress — live percentage bar in Downloads queue and manga detail view

**Reader**
✅ Light novel reader (text-based, Markdown)
🔳 Novel reader typography controls (font size, line width, serif/sans)
🔳 Keyboard and remote shortcuts in web reader
🔳 Reading statistics (time spent, chapters per week)

**Library**
🔳 Metadata editing (title, cover, tags)
🔳 CBZ / CBR local import
🔳 Backup and export (library + reading progress)

**Server / Observability**
✅ In-memory log ring buffer — `GET /api/logs` streams recent server events to the web UI
✅ Runtime log level control — Settings → Logs → adjust capture level without restart
🔳 Structured log export (JSON download)

**Integrations**
🔳 Push notifications for new chapters
🔳 Webhook on new chapter download

**Infrastructure**
🔳 PostgreSQL support alongside SQLite
