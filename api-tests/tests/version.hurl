# GET /api/version — no auth required
GET {{base}}/api/version
HTTP 200
[Asserts]
jsonpath "$.version" isString
jsonpath "$.version" matches /^\d+\.\d+\.\d+$/
