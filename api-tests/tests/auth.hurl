# ── Auth status ──────────────────────────────────────────────────────────────
# Server is already registered (setup script ran first)
GET {{base}}/api/auth/status
HTTP 200
[Asserts]
jsonpath "$.registered" == true


# ── Login with bad password → 401 ────────────────────────────────────────────
POST {{base}}/api/auth/login
Content-Type: application/json
{
  "username": "test-admin",
  "password": "wrongpassword"
}
HTTP 401


# ── Login success ─────────────────────────────────────────────────────────────
POST {{base}}/api/auth/login
Content-Type: application/json
{
  "username": "test-admin",
  "password": "testpassword123"
}
HTTP 200
[Asserts]
jsonpath "$.token" isString
jsonpath "$.role" == "admin"
jsonpath "$.username" == "test-admin"


# ── Protected endpoint without token → 401 ───────────────────────────────────
GET {{base}}/api/auth/me
HTTP 401


# ── Protected endpoint with bad token → 401 ──────────────────────────────────
GET {{base}}/api/auth/me
Authorization: Bearer notavalidtoken
HTTP 401


# ── /me with valid token ──────────────────────────────────────────────────────
GET {{base}}/api/auth/me
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$.username" == "test-admin"
jsonpath "$.role" == "admin"
jsonpath "$.id" isString


# ── PATCH /me — change allow_explicit ────────────────────────────────────────
PATCH {{base}}/api/auth/me
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "allow_explicit": false
}
HTTP 200
[Asserts]
jsonpath "$.allow_explicit" == false
