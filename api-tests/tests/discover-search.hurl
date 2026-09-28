# ── Search — unauthenticated → 401 ────────────────────────────────────────────
GET {{base}}/api/discover?q=test
HTTP 401


# spec: 004/FR-001
# ── Search — known-good fixture query → 200 array ─────────────────────────────
# "Fixture Manga" is the same query e2e/tests/discover.spec.ts relies on — it
# resolves through the fixture plugin-host in docker-compose.test.yml, so this
# doesn't depend on any real external metadata site being reachable/unchanged.
GET {{base}}/api/discover?q=Fixture%20Manga
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection


# ── Add — unauthenticated → 401 ───────────────────────────────────────────────
POST {{base}}/api/discover/add
Content-Type: application/json
{
  "title": "Hurl Test Title",
  "mangaupdates_id": "0",
  "description": null,
  "cover_url": null,
  "status": "ongoing",
  "content_type": "manga",
  "tags": null,
  "author": null,
  "year": null
}
HTTP 401


# spec: 004/FR-014
# ── Add — creates a title ─────────────────────────────────────────────────────
POST {{base}}/api/discover/add
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "title": "Hurl Test Title",
  "mangaupdates_id": "0",
  "description": null,
  "cover_url": null,
  "status": "ongoing",
  "content_type": "manga",
  "tags": null,
  "author": null,
  "year": null
}
HTTP 200
[Captures]
added_id: jsonpath "$.id"
[Asserts]
jsonpath "$.title" == "Hurl Test Title"
jsonpath "$.content_type" == "manga"


# ── Cleanup ────────────────────────────────────────────────────────────────────
DELETE {{base}}/api/titles/{{added_id}}?delete_files=true
Authorization: Bearer {{token}}
HTTP 204
