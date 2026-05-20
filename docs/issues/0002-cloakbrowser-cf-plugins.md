# 0002 — CloakBrowser sidecar + CF-dependent plugin migrations (toonily, comick, novelfull)

**Status**: done
**Labels**: enhancement, ready-for-agent
**Blocked by**: 0001

## What to build

Wire CloakBrowser as the Cloudflare bypass sidecar and migrate the three CF-dependent plugins to bundle format using `PluginContext.getBrowser()`.

**docker-compose**:
- Add `cloakbrowser` service using `cloakhq/cloakbrowser` image in `cloakserve` CDP mode
- Remove `flaresolverr` service
- Pass `CLOAKBROWSER_WS_URL` env var to `plugin-host`

**Plugin Host update**:
- On startup, connect to CloakBrowser via CDP using `CLOAKBROWSER_WS_URL`
- Implement `PluginContext.getBrowser()` — returns a Playwright `Browser` instance backed by the CloakBrowser CDP connection
- Pool/reuse browser contexts across plugin calls; handle reconnect on disconnect

**toonily, comick, novelfull migrations**:
- Strip Express server and `FLARESOLVERR_URL` logic from each plugin
- Replace FlareSolverr HTTP calls with `PluginContext.getBrowser()` + Playwright navigation
- Export `info`, `init(ctx)`, `search(q)`, `chapters(id)`, `pages(id)`, and optional `trending()`, `meta(id)` from each
- Each compiles to a single `.js` bundle via esbuild

## Acceptance criteria

- [ ] `docker compose up` starts with `cloakbrowser` present and `flaresolverr` absent
- [ ] toonily search returns results through plugin-host
- [ ] comick search returns results through plugin-host
- [ ] novelfull chapter text loads through plugin-host
- [ ] `PluginContext.getBrowser()` reconnects automatically if CloakBrowser restarts
