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
# {{token}} (the API test stack's admin) is explicit-allowed (GH #219: the file this
# comment used to describe as "{{token}} is non-explicit" was never actually run, so
# this was never caught — the shared admin token can't be relied on for this
# assertion, and other .hurl files run in parallel so an incidental PATCH to it
# elsewhere isn't ordering-safe either. A fresh member (admin-created users default
# to allow_explicit=false) is self-contained instead.
POST {{base}}/api/users
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "username": "trending-test-member",
  "password": "testpassword123"
}
HTTP 201

POST {{base}}/api/auth/login
Content-Type: application/json
{
  "username": "trending-test-member",
  "password": "testpassword123"
}
HTTP 200
[Captures]
member_token: jsonpath "$.token"
[Asserts]
jsonpath "$.allow_explicit" == false

GET {{base}}/api/discover/trending/adult-manhwa
Authorization: Bearer {{member_token}}
HTTP 403
