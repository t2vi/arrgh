# 0003 — Migrate remaining plugins + remove standalone containers

**Status**: done
**Labels**: enhancement, ready-for-agent
**Blocked by**: 0001, 0002

## What to build

Migrate mangapill and royalroad to bundle format, bake all 6 bundles into the plugin-host image, and remove the 6 standalone plugin containers from docker-compose. Final state: 3 containers total.

**mangapill migration**:
- Strip Express server; keep scraping logic and `fetchCoverBytes` in `mangapill.ts` unchanged
- Export `info`, `init(ctx)`, `search(q)`, `chapters(id)`, `pages(id)`, `meta(id)`, `cover(url)` from `index.ts`
- No CloakBrowser needed — uses direct fetch with `User-Agent` + `Referer` headers

**royalroad migration**:
- Strip Express server; keep scraping logic and Turndown conversion unchanged
- Export `info`, `init(ctx)`, `search(q)`, `chapters(id)`, `text(id)`, `meta(id)` from `index.ts`
- `chapter_format: 'text'` in chapter results

**Plugin Host image**:
- Bake all 6 compiled bundles into the image at build time under `bundles/`
- `LANGUAGES` env var passed to mangadex/comick bundles via PluginContext or direct env read

**docker-compose cleanup**:
- Remove `mangadex`, `mangapill`, `toonily`, `comick`, `royalroad`, `novelfull` services
- Remove `flaresolverr` if not already removed in 0002
- Final compose: `arrgh`, `plugin-host`, `cloakbrowser`, `arrgh_data` volume

## Acceptance criteria

- [ ] `docker compose up` starts exactly 3 containers
- [ ] All 6 sources return search results through plugin-host
- [ ] royalroad novel chapter text downloads and renders correctly
- [ ] mangapill cover images load correctly (Referer header preserved)
- [ ] No references to flaresolverr or standalone plugin services remain in docker-compose.yml
