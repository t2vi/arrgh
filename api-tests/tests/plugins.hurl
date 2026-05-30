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
