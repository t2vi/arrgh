# 0004 — Plugin Index + Browse & Install UI

**Status**: done
**Labels**: enhancement, ready-for-agent
**Blocked by**: 0003

## What to build

Create the community plugin index, add an install endpoint to plugin-host, and build Settings → Sources → Browse so admins can install plugins from within the app.

**Plugin index** (`docs/issues/plugin-index/index.json` or a separate repo):
- `index.json` listing available plugins: `[{ id, name, description, version, download_url, default_explicit, content_types }]`
- Seed with the 6 default plugins
- `README.md` explaining how to submit a community plugin

**Plugin Host — install/remove endpoints**:
- `POST /plugins/install { url: string }` — downloads bundle, writes to community bundles volume, hot-loads it
- Community bundles volume is separate from the baked-in defaults directory so installs survive restarts
- `DELETE /plugins/:id` — unloads and removes a community bundle

**Rust server**:
- `GET /api/plugins/index` — fetches the plugin index URL (default hardcoded, overridable via settings) and returns parsed JSON
- `POST /api/plugins/install { plugin_id }` — looks up plugin in index, calls plugin-host install, registers source in Source Registry
- `DELETE /api/plugins/:id` — calls plugin-host delete, removes from Source Registry

**Settings UI** (`web/src/features/settings/`):
- New "Sources" tab listing installed sources with enable/disable toggle
- Uninstall button for community plugins only (bundled defaults cannot be uninstalled)
- "Browse" button fetches index, shows install cards for plugins not yet installed
- Install button triggers `POST /api/plugins/install`, shows loading state, refreshes list on success

## Acceptance criteria

- [ ] Plugin index exists with all 6 default plugins listed
- [ ] Admin can open Settings → Sources → Browse and see available community plugins
- [ ] Admin can install a plugin from Browse UI; it appears in source list and works in Discover immediately
- [ ] Installed community plugins persist across plugin-host restarts
- [ ] Bundled default plugins listed in Sources without uninstall button
