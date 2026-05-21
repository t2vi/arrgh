use axum::{response::{Html, Json}, routing::get, Router};
use serde_json::{json, Value};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/docs", get(scalar_ui))
        .route("/openapi.json", get(openapi_spec))
}

async fn scalar_ui() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html>
<head>
  <title>*ARRgh API</title>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
</head>
<body>
  <script id="api-reference" data-url="/api/openapi.json"></script>
  <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"#)
}

async fn openapi_spec() -> Json<Value> {
    Json(json!({
      "openapi": "3.1.0",
      "info": {
        "title": "*ARRgh",
        "description": "Self-hosted manga manager & downloader",
        "version": "0.1.0",
        "license": {
          "name": "GNU GPL v3",
          "url": "https://www.gnu.org/licenses/gpl-3.0.html"
        }
      },
      "servers": [{ "url": "http://localhost:3000" }],
      "paths": {
        "/api/manga": {
          "get": {
            "tags": ["Library"],
            "summary": "List manga",
            "parameters": [
              { "name": "page",   "in": "query", "schema": { "type": "integer" } },
              { "name": "search", "in": "query", "schema": { "type": "string"  } }
            ],
            "responses": { "200": { "description": "Paginated manga list" } }
          }
        },
        "/api/manga/{id}": {
          "get": {
            "tags": ["Library"],
            "summary": "Get manga by ID",
            "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "200": { "description": "Manga object" }, "404": { "description": "Not found" } }
          },
          "delete": {
            "tags": ["Library"],
            "summary": "Remove manga from library",
            "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "204": { "description": "Deleted" } }
          }
        },
        "/api/manga/{id}/sync": {
          "post": {
            "tags": ["Library"],
            "summary": "Sync chapters from MangaDex",
            "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "202": { "description": "Sync started" } }
          }
        },
        "/api/chapters/manga/{manga_id}": {
          "get": {
            "tags": ["Chapters"],
            "summary": "List chapters for a manga",
            "parameters": [{ "name": "manga_id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "200": { "description": "Chapter list" } }
          }
        },
        "/api/chapters/{id}/download": {
          "post": {
            "tags": ["Chapters"],
            "summary": "Queue a chapter for download",
            "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "202": { "description": "Queued" } }
          }
        },
        "/api/queue": {
          "get": {
            "tags": ["Queue"],
            "summary": "List all download queue items",
            "responses": { "200": { "description": "Queue items" } }
          }
        },
        "/api/queue/manga/{manga_id}": {
          "get": {
            "tags": ["Queue"],
            "summary": "Queue items for a specific manga",
            "parameters": [{ "name": "manga_id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "200": { "description": "Queue items" } }
          }
        },
        "/api/queue/{id}": {
          "delete": {
            "tags": ["Queue"],
            "summary": "Remove / cancel a queue item",
            "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "204": { "description": "Removed" } }
          }
        },
        "/api/progress/{chapter_id}": {
          "get": {
            "tags": ["Progress"],
            "summary": "Get read progress for a chapter",
            "parameters": [{ "name": "chapter_id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "200": { "description": "ReadProgress" }, "404": { "description": "No progress yet" } }
          },
          "put": {
            "tags": ["Progress"],
            "summary": "Update read progress",
            "parameters": [{ "name": "chapter_id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "requestBody": {
              "required": true,
              "content": { "application/json": { "schema": {
                "type": "object",
                "properties": {
                  "current_page": { "type": "integer" },
                  "completed":    { "type": "boolean" }
                }
              }}}
            },
            "responses": { "200": { "description": "Updated progress" } }
          }
        },
        "/api/progress/manga/{manga_id}": {
          "get": {
            "tags": ["Progress"],
            "summary": "Get all read progress for a manga",
            "parameters": [{ "name": "manga_id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "200": { "description": "Progress list" } }
          }
        },
        "/api/discover": {
          "get": {
            "tags": ["Discover"],
            "summary": "Search MangaDex",
            "parameters": [{ "name": "q", "in": "query", "required": true, "schema": { "type": "string" } }],
            "responses": { "200": { "description": "Search results" } }
          }
        },
        "/api/discover/add": {
          "post": {
            "tags": ["Discover"],
            "summary": "Add manga to library from MangaDex",
            "responses": { "200": { "description": "Added manga" } }
          }
        },
        "/api/media/page/{chapter_id}/{page}": {
          "get": {
            "tags": ["Media"],
            "summary": "Get a page image (local file or 307 redirect to CDN)",
            "parameters": [
              { "name": "chapter_id", "in": "path", "required": true, "schema": { "type": "string" } },
              { "name": "page",       "in": "path", "required": true, "schema": { "type": "integer" } }
            ],
            "responses": { "200": { "description": "Image" }, "307": { "description": "Redirect to CDN" } }
          }
        },
        "/api/media/cover/{manga_id}": {
          "get": {
            "tags": ["Media"],
            "summary": "Get cover image (local file or 307 redirect)",
            "parameters": [{ "name": "manga_id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "200": { "description": "Cover image" }, "307": { "description": "Redirect" } }
          }
        },
        "/api/logs": {
          "get": {
            "tags": ["Logs"],
            "summary": "Get recent log entries from in-memory ring buffer",
            "parameters": [
              { "name": "limit", "in": "query", "schema": { "type": "integer", "default": 200, "maximum": 500 } }
            ],
            "responses": { "200": { "description": "Array of LogEntry" } }
          }
        },
        "/api/logs/level": {
          "get": {
            "tags": ["Logs"],
            "summary": "Get current ring buffer capture level",
            "responses": { "200": { "description": "{ level: string }" } }
          },
          "patch": {
            "tags": ["Logs"],
            "summary": "Set ring buffer capture level (admin only)",
            "requestBody": {
              "required": true,
              "content": { "application/json": { "schema": { "type": "object", "properties": { "level": { "type": "string", "enum": ["ERROR", "WARN", "INFO", "DEBUG"] } } } } }
            },
            "responses": { "204": { "description": "Updated" }, "403": { "description": "Forbidden" }, "422": { "description": "Invalid level" } }
          }
        }
      }
    }))
}
