# ── List titles — empty library ───────────────────────────────────────────────
GET {{base}}/api/titles
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" == 0
jsonpath "$.page" == 1


# ── Pagination params accepted ────────────────────────────────────────────────
GET {{base}}/api/titles?page=1&per_page=20
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection


# ── New releases — empty ──────────────────────────────────────────────────────
GET {{base}}/api/titles/new-releases
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection


# ── Get unknown title → 404 ───────────────────────────────────────────────────
GET {{base}}/api/titles/nonexistent-id
Authorization: Bearer {{token}}
HTTP 404


# ── Remove unknown title → 404 ────────────────────────────────────────────────
DELETE {{base}}/api/titles/nonexistent-id
Authorization: Bearer {{token}}
HTTP 404
