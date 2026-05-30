# ── GET /api/settings ────────────────────────────────────────────────────────
GET {{base}}/api/settings
HTTP 200
[Asserts]
jsonpath "$.download_dir" isString
jsonpath "$.plugin_host_url" isString


# ── POST /api/settings — save and read back ───────────────────────────────────
POST {{base}}/api/settings
Content-Type: application/json
{
  "download_dir": "/data/downloads",
  "plugin_host_url": "http://plugin-host:4000"
}
HTTP 200


# ── Confirm saved values are reflected ───────────────────────────────────────
GET {{base}}/api/settings
HTTP 200
[Asserts]
jsonpath "$.download_dir" == "/data/downloads"
