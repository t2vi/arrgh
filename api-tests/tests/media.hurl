# media routes carry no auth (matches the prior .NET behavior) — see
# server/src/api/media.rs's module doc.

# spec: 006/FR-005, 006/FR-012
# ── Page — unknown chapter → 404 ──────────────────────────────────────────────
GET {{base}}/api/media/page/nonexistent-chapter-id/0
HTTP 404


# spec: 006/FR-007, 006/FR-012
# ── Cover — unknown title → 404 ───────────────────────────────────────────────
GET {{base}}/api/media/cover/nonexistent-title-id
HTTP 404


# ── Meta-cover — unknown key → 404 ────────────────────────────────────────────
GET {{base}}/api/media/meta-cover?key=nonexistent-key
HTTP 404


# ── Proxy — unreachable url → 502 ─────────────────────────────────────────────
GET {{base}}/api/media/proxy?url=http://127.0.0.1:1/unreachable
HTTP 502


# spec: 006/FR-007
# ── Cover — a real title with no cover set → 404 (no cover, no meta-cover) ────
POST {{base}}/api/discover/add
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "title": "Hurl Media Test Title",
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
media_title_id: jsonpath "$.id"

GET {{base}}/api/media/cover/{{media_title_id}}
HTTP 404

DELETE {{base}}/api/titles/{{media_title_id}}?delete_files=true
Authorization: Bearer {{token}}
HTTP 204
