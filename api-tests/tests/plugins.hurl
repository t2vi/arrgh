# ── GET /api/plugins/index ───────────────────────────────────────────────────
GET {{base}}/api/plugins/index
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
# Each plugin entry has required fields
jsonpath "$[0].id" isString
jsonpath "$[0].name" isString
jsonpath "$[0].bundled" == true


# ── Install with no URL → 400 ────────────────────────────────────────────────
POST {{base}}/api/plugins/install
Authorization: Bearer {{token}}
Content-Type: application/json
{}
HTTP 400


# ── Delete bundled plugin → 403 ──────────────────────────────────────────────
DELETE {{base}}/api/plugins/mangadex
Authorization: Bearer {{token}}
HTTP 403


# ── spec 031: plugin status + update/revert ─────────────────────────────────
# spec: 031/FR-003
GET {{base}}/api/plugins
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$.catalog" == "live"
jsonpath "$.plugins" isCollection
jsonpath "$.plugins[0].id" isString
jsonpath "$.plugins[0].origin" isString


# Every bundled entry has a real download_url + sha256 since spec 031 phase C, so
# `blocked_reason`'s "no download"/"no checksum" branches (unit-tested with synthetic
# data in server/src/plugins.rs) aren't reachable from live catalog data any more —
# this stack's `fixture` plugin-host double doesn't implement the host-protocol
# endpoint the next check needs, so update is refused for that reason instead.
# spec: 031/FR-005
POST {{base}}/api/plugins/mangadex/update
Authorization: Bearer {{token}}
HTTP 422
[Asserts]
jsonpath "$.error" contains "unreachable"


# Unknown id → 404
POST {{base}}/api/plugins/no-such-plugin/update
Authorization: Bearer {{token}}
HTTP 404
