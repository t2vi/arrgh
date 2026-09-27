# nhentai (bundled fallback)

Source of truth for this plugin moved to
[t2vi/arrgh-plugin-nhentai](https://github.com/t2vi/arrgh-plugin-nhentai) (spec 031 phase C,
ADR 0034/0035). Do not edit `src/` here — this copy only exists so the arrgh Docker image ships a
working bundled version; it is overridden by whatever `plugin-index/index.json`'s `download_url`
points admins at in Settings → Sources → Plugins.

Sync this copy from the standalone repo (rare — only when the standalone version needs to become
the new bundled floor) rather than developing against it directly.
