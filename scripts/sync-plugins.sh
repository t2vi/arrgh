#!/usr/bin/env bash
# Fetch the pinned bundled plugin set (spec 031 phase D) into plugin-host/bundles/ (what the dev
# plugin-host loads; it hot-reloads changes) — same fetch-and-verify path the Docker image build
# uses, so local dev runs exactly what ships. To test an in-progress change to a plugin, build it
# in its own arrgh-plugin-<id> repo and drop the bundle into plugin-host/community-bundles/
# instead (see that repo's README) — this script never touches community-bundles/.
set -euo pipefail
cd "$(dirname "$0")/.."

node scripts/fetch-plugin-bundles.mjs --out plugin-host/bundles
