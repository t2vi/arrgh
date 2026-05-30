# ── List sources — empty ─────────────────────────────────────────────────────
GET {{base}}/api/sources
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection


# ── Add a source ──────────────────────────────────────────────────────────────
POST {{base}}/api/sources
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "name": "MangaDex",
  "base_url": "http://plugin-host:4000",
  "source_key": "mangadex",
  "content_types": ["manga", "manhwa", "manhua"],
  "enabled": true,
  "priority": 1
}
HTTP 200
[Captures]
source_id: jsonpath "$.id"
[Asserts]
jsonpath "$.name" == "MangaDex"
jsonpath "$.enabled" == true


# ── List — now has 1 ──────────────────────────────────────────────────────────
GET {{base}}/api/sources
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" count == 1


# ── Patch — disable the source ───────────────────────────────────────────────
PATCH {{base}}/api/sources/{{source_id}}
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "enabled": false
}
HTTP 200
[Asserts]
jsonpath "$.enabled" == false


# ── Delete the source ─────────────────────────────────────────────────────────
DELETE {{base}}/api/sources/{{source_id}}
Authorization: Bearer {{token}}
HTTP 204


# ── List — empty again ────────────────────────────────────────────────────────
GET {{base}}/api/sources
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" count == 0
