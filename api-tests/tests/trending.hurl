# ── Trending lanes (ADR 0032) — unauthenticated → 401 ─────────────────────────
GET {{base}}/api/discover/trending/manga
HTTP 401

GET {{base}}/api/discover/trending/manhwa
HTTP 401

GET {{base}}/api/discover/trending/manhua
HTTP 401

GET {{base}}/api/discover/trending/adult-manhwa
HTTP 401


# ── /trending/manga → 200 array ───────────────────────────────────────────────
GET {{base}}/api/discover/trending/manga
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection


# ── /trending/manhwa → 200 array ──────────────────────────────────────────────
GET {{base}}/api/discover/trending/manhwa
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection


# ── /trending/manhua → 200 array ──────────────────────────────────────────────
GET {{base}}/api/discover/trending/manhua
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection


# ── /trending/adult-manhwa → 403 for non-explicit user ────────────────────────
# ({{token}} is a non-explicit user in the API test stack)
GET {{base}}/api/discover/trending/adult-manhwa
Authorization: Bearer {{token}}
HTTP 403
