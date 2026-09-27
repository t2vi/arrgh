//! Content types and chapter formats (GH #163). Columns stay TEXT; these are
//! the only values the server writes or compares. External vocabularies
//! (AniList `MANHWA`, MangaUpdates "web novel", …) are parsed where they
//! arrive and mapped onto these.

pub const MANGA: &str = "manga";
pub const MANHWA: &str = "manhwa";
pub const MANHUA: &str = "manhua";
pub const NOVEL: &str = "novel";
pub const HENTAI: &str = "hentai";
/// Every content type a title can have (admin PATCH validates against this).
pub const CONTENT_TYPES: [&str; 5] = [MANGA, MANHWA, MANHUA, NOVEL, HENTAI];

/// `chapters.chapter_format`: page images (manga-likes) or Markdown text (novels).
pub const FORMAT_PAGES: &str = "pages";
pub const FORMAT_TEXT: &str = "text";

/// Chapter format for a title's content type.
pub fn chapter_format_for(content_type: &str) -> &'static str {
    if content_type == NOVEL {
        FORMAT_TEXT
    } else {
        FORMAT_PAGES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn novels_are_text_everything_else_pages() {
        assert_eq!(chapter_format_for(NOVEL), FORMAT_TEXT);
        for ct in [MANGA, MANHWA, MANHUA, HENTAI] {
            assert_eq!(chapter_format_for(ct), FORMAT_PAGES);
        }
    }
}
