use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Manga {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub local_path: Option<String>,
    pub author: Option<String>,
    pub year: Option<i64>,
    pub tags: Option<String>,
    pub sync_status: String,
    pub content_type: String,
    pub is_explicit: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MangaSource {
    pub id: String,
    pub manga_id: String,
    pub source: String,
    pub source_id: String,
    pub discovered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Chapter {
    pub id: String,
    pub manga_id: String,
    pub title: Option<String>,
    pub number: f64,
    pub volume: Option<f64>,
    pub local_path: Option<String>,
    pub page_count: i64,
    pub downloaded: bool,
    pub has_sources: bool,
    pub chapter_format: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChapterSource {
    pub id: String,
    pub chapter_id: String,
    pub source: String,
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReadProgress {
    pub id: String,
    pub user_id: String,
    pub chapter_id: String,
    pub current_page: i64,
    pub completed: bool,
    pub updated_at: DateTime<Utc>,
}
