# ── GET /api/logs ─────────────────────────────────────────────────────────────
GET {{base}}/api/logs
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$.entries" isCollection
jsonpath "$.level" isString


# ── GET /api/logs/level ───────────────────────────────────────────────────────
GET {{base}}/api/logs/level
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$.level" isString


# ── PATCH /api/logs/level — set to debug ──────────────────────────────────────
PATCH {{base}}/api/logs/level
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "level": "debug"
}
HTTP 200


# ── Restore to info ───────────────────────────────────────────────────────────
PATCH {{base}}/api/logs/level
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "level": "info"
}
HTTP 200
