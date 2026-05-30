# ── List sources — seeded on first boot ──────────────────────────────────────
GET {{base}}/api/sources
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$" count > 0
jsonpath "$[0].id" isString
jsonpath "$[0].priority" isInteger
[Captures]
source_id: jsonpath "$[0].id"


# ── Patch — disable + update priority ────────────────────────────────────────
PATCH {{base}}/api/sources/{{source_id}}
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "enabled": false,
  "priority": 999
}
HTTP 204


# ── Re-enable — cleanup ───────────────────────────────────────────────────────
PATCH {{base}}/api/sources/{{source_id}}
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "enabled": true
}
HTTP 204


# ── POST — stubbed (ADR 0030 plugin-host wiring not yet ported) ───────────────
POST {{base}}/api/sources
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "base_url": "http://localhost:9999"
}
HTTP 502


# ── Patch nonexistent → 404 ───────────────────────────────────────────────────
PATCH {{base}}/api/sources/nonexistent-id
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "enabled": false
}
HTTP 404


# ── Delete nonexistent → 404 ─────────────────────────────────────────────────
DELETE {{base}}/api/sources/nonexistent-id
Authorization: Bearer {{token}}
HTTP 404
