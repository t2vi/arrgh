# The upsert-success round trip (PUT creating/updating a real progress row)
# needs a real chapter to satisfy read_progress's FK on chapter_id — that's
# covered at the integration layer instead (server/tests/progress.rs, seeded
# chapter). This file covers what's provable without seeded data: auth
# gating and the read-only not-found/empty-list paths.

# spec: 006/FR-012
# ── Get progress — unauthenticated → 401 ──────────────────────────────────────
GET {{base}}/api/progress/some-chapter-id
HTTP 401


# spec: 006/FR-012
# ── Update progress — unauthenticated → 401 ───────────────────────────────────
PUT {{base}}/api/progress/some-chapter-id
Content-Type: application/json
{
  "current_page": 0,
  "completed": false
}
HTTP 401


# spec: 006/FR-012
# ── Continue reading — unauthenticated → 401 ──────────────────────────────────
GET {{base}}/api/progress/continue
HTTP 401


# spec: 006/FR-011
# ── Continue reading — empty ──────────────────────────────────────────────────
GET {{base}}/api/progress/continue
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection


# ── Get progress — no row for this chapter → 404 ──────────────────────────────
GET {{base}}/api/progress/nonexistent-chapter-id
Authorization: Bearer {{token}}
HTTP 404


# spec: 006/FR-012
# ── Title progress list — unauthenticated → 401 ───────────────────────────────
GET {{base}}/api/progress/title/nonexistent-title-id
HTTP 401


# ── Title progress list — no matching chapter row → empty ────────────────────
GET {{base}}/api/progress/title/nonexistent-title-id
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$" count == 0
