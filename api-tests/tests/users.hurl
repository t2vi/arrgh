# spec: 003/FR-008
# ── Unauthenticated → 401 ─────────────────────────────────────────────────────
GET {{base}}/api/users
HTTP 401


# ── List users (admin) ────────────────────────────────────────────────────────
GET {{base}}/api/users
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.username=='test-admin')].username" == "test-admin"


# ── Own id, for the "can't delete self" check below ───────────────────────────
GET {{base}}/api/auth/me
Authorization: Bearer {{token}}
HTTP 200
[Captures]
admin_id: jsonpath "$.id"


# spec: 003/FR-010
# ── Create — invalid password (too short) → 422 ───────────────────────────────
POST {{base}}/api/users
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "username": "users-test-short-pw",
  "password": "abc"
}
HTTP 422


# ── Create — member ───────────────────────────────────────────────────────────
POST {{base}}/api/users
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "username": "users-test-member",
  "password": "testpassword123"
}
HTTP 201


# spec: 003/FR-010
# ── Create — duplicate username → 409 ─────────────────────────────────────────
POST {{base}}/api/users
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "username": "users-test-member",
  "password": "testpassword123"
}
HTTP 409


# spec: 003/FR-009
# ── New member's id, from the list ────────────────────────────────────────────
GET {{base}}/api/users
Authorization: Bearer {{token}}
HTTP 200
[Captures]
member_id: jsonpath "$[?(@.username=='users-test-member')].id"
[Asserts]
jsonpath "$[?(@.username=='users-test-member')].role" == "member"
jsonpath "$[?(@.username=='users-test-member')].allow_explicit" == false


# spec: 003/FR-008
# ── A member token can't reach admin-only routes → 403 ────────────────────────
POST {{base}}/api/auth/login
Content-Type: application/json
{
  "username": "users-test-member",
  "password": "testpassword123"
}
HTTP 200
[Captures]
member_token: jsonpath "$.token"

GET {{base}}/api/users
Authorization: Bearer {{member_token}}
HTTP 403


# spec: 003/FR-011
# ── Patch — invalid role → 422 ─────────────────────────────────────────────────
PATCH {{base}}/api/users/{{member_id}}
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "role": "superadmin"
}
HTTP 422


# spec: 003/FR-013
# ── Patch — unknown user → 404 ─────────────────────────────────────────────────
PATCH {{base}}/api/users/nonexistent-id
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "allow_explicit": true
}
HTTP 404


# spec: 003/FR-008
# ── Patch — role + allow_explicit + password ──────────────────────────────────
PATCH {{base}}/api/users/{{member_id}}
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "role": "admin",
  "allow_explicit": true,
  "password": "newpassword123"
}
HTTP 204

GET {{base}}/api/users
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$[?(@.username=='users-test-member')].role" == "admin"
jsonpath "$[?(@.username=='users-test-member')].allow_explicit" == true

# The password change took — old password now rejected, new one works
POST {{base}}/api/auth/login
Content-Type: application/json
{
  "username": "users-test-member",
  "password": "testpassword123"
}
HTTP 401

POST {{base}}/api/auth/login
Content-Type: application/json
{
  "username": "users-test-member",
  "password": "newpassword123"
}
HTTP 200


# spec: 003/FR-012
# ── Delete self → 403 ──────────────────────────────────────────────────────────
DELETE {{base}}/api/users/{{admin_id}}
Authorization: Bearer {{token}}
HTTP 403


# spec: 003/FR-013
# ── Delete — unknown user → 404 ────────────────────────────────────────────────
DELETE {{base}}/api/users/nonexistent-id
Authorization: Bearer {{token}}
HTTP 404


# ── Delete — cleanup the test member ───────────────────────────────────────────
DELETE {{base}}/api/users/{{member_id}}
Authorization: Bearer {{token}}
HTTP 204
