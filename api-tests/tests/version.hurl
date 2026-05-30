# GET /api/version — no auth required
GET {{base}}/api/version
HTTP 200
[Asserts]
jsonpath "$.current" isString
jsonpath "$.current" matches /^\d+\.\d+\.\d+$/
