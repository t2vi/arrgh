# *ARRgh!

A self-hosted manga manager, downloader, and reader for your home server. Supports manga, manhwa, and manhua from online sources. Built to run on a NAS, Raspberry Pi, or any always-on box.

> I'm a solo dev who built this for myself — tired of juggling browser tabs, download scripts, and folder structures just to keep up with series. If you find it useful or want to contribute, you're very welcome. See [Contributing](#contributing).

---

## Features

- Browse and search manga from supported sources
- Download chapters to your server for offline reading
- Web reader (paged or scroll mode) and Flutter app (Android / Firestick / tablet)
- Multi-user support — per-user libraries with shared file storage, per-user reading progress
- Auto-download new chapters on a schedule
- Explicit content controls — admin grants access per user

---

## Quick start (Docker)

```bash
docker run -d \
  --name arrgh \
  -p 8080:8080 \
  -v arrgh_data:/data \
  ghcr.io/t2vi/arrgh:latest
```

Open `http://<your-server-ip>:8080` — the setup wizard runs on first launch.

Or with Compose:

```bash
curl -O https://raw.githubusercontent.com/t2vi/arrgh/main/docker-compose.yml
docker compose up -d
```

See [docs/deploy/docker-compose.md](docs/deploy/docker-compose.md) for configuration options.

---

## Architecture

```
arrgh/
├── server/   # Rust / Axum API server
├── web/      # React + TypeScript SPA
└── app/      # Flutter app (Android / Firestick / tablet)
```

- **Backend**: Rust, Axum, SQLx (SQLite), Tokio
- **Frontend**: React 18, TypeScript, Vite, TanStack Query, Tailwind
- **Mobile**: Flutter, Riverpod

---

## Contributing

Issues and PRs are welcome. A few things to know:

- This is a personal project — I may be slow to review, but I do look at everything
- Check open issues before starting large features; comment to claim one
- Run `cargo test` (server) and `flutter test` (app) before submitting
- Follow the existing code style — see [server/README.md](server/README.md) and [web/README.md](web/README.md) for dev setup

No CLA, no process overhead. Just open a PR.

---

## Roadmap

**Sources**
- [ ] Additional manga/manhwa/manhua sources
- [ ] Manhwa-specific source behaviour with vertical/webtoon reader images
- [ ] Community source plugins

**Reader**
- [ ] Light novel reader (text-based, EPUB/TXT support)
- [ ] Keyboard and remote shortcuts in web reader
- [ ] Reading statistics (time spent, chapters per week)

**Library**
- [ ] Metadata editing (title, cover, tags)
- [ ] CBZ / CBR local import
- [ ] Backup and export (library + reading progress)

**Integrations**
- [ ] Push notifications for new chapters
- [ ] Webhook on new chapter download

**Infrastructure**
- [ ] PostgreSQL support alongside

These are just the items I listed down I think would be important — open one to propose or claim.

---

## License

[GNU GPL v3](LICENSE)
