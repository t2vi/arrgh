# ── List queue — empty ───────────────────────────────────────────────────────
GET {{base}}/api/queue
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection


# ── Clear completed — idempotent on empty queue ───────────────────────────────
DELETE {{base}}/api/queue/completed
Authorization: Bearer {{token}}
HTTP 204


# ── Remove unknown item → 404 ────────────────────────────────────────────────
DELETE {{base}}/api/queue/nonexistent-id
Authorization: Bearer {{token}}
HTTP 404
