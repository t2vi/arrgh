# *ARRgh Plugin Index

This directory is the default plugin registry served at startup. The Rust server reads `PLUGIN_INDEX_URL` (defaults to this file via raw GitHub) when an admin browses or installs community plugins.

## Schema — `index.json`

Each entry:

| Field | Type | Description |
|---|---|---|
| `id` | string | Matches the bundle filename (`<id>.js`) and plugin-host route prefix |
| `name` | string | Display name |
| `description` | string | Short one-liner shown in browse UI |
| `version` | string | Semver |
| `download_url` | string \| null | URL to the compiled `.js` bundle. `null` for bundled defaults |
| `bundled` | boolean \| null | `true` = ships inside the plugin-host container, can't be deleted |
| `default_explicit` | boolean | Whether the source has adult content |
| `content_types` | string[] | `"manga"` and/or `"novel"` |

## Submitting a Community Plugin

1. Build your plugin as a single-file CommonJS bundle (see `plugins/mangadex/` for reference).
2. Host the `.js` file at a stable public URL (GitHub Releases, CDN, etc.).
3. Open a PR adding your entry to `index.json` with `"bundled": false` and a valid `download_url`.
4. The bundle must export: `info`, `init(ctx)`, `search(q)`, `chapters(id, langs)`.
5. Optional exports: `pages(id)`, `trending()`, `meta(id, langs)`, `cover(url)`, `chapterText(id)`.

Community plugins are installed into a writable volume at runtime and can be removed by an admin.
