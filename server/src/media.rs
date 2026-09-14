//! Page/image serving support — port of `Api/Media.cs`'s pure helpers and
//! DB access (ADR 0033, S8 #130). No auth on any of this group's routes,
//! same as .NET (`MapMediaRoutes` has no `.RequireAuthorization()` calls
//! and `api` itself carries none) — `<img>` tags can't send an
//! `Authorization` header, so these endpoints are deliberately public.
//!
//! Title-key normalization reuses `crate::discover::normalize_title`
//! rather than a second copy — this module and Discover's cover cache
//! must agree on the same key or `title_meta` lookups silently miss.

use sqlx::SqlitePool;

// ── pure helpers ─────────────────────────────────────────────────────────

pub fn detect_content_type(data: &[u8]) -> Option<&'static str> {
    if data.len() < 4 {
        return None;
    }
    if data[0] == 0xFF && data[1] == 0xD8 {
        return Some("image/jpeg");
    }
    if data[0] == 0x89 && data[1] == 0x50 && data[2] == 0x4E && data[3] == 0x47 {
        return Some("image/png");
    }
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if &data[0..4] == b"GIF8" {
        return Some("image/gif");
    }
    if data.len() >= 8 && &data[4..8] == b"ftyp" {
        return Some("image/avif");
    }
    None
}

/// Drops an APP2 `ICC_PROFILE` segment from a JPEG (some readers choke on
/// large embedded profiles); anything else passes through byte-identical.
pub fn strip_jpeg_icc(data: &[u8]) -> Vec<u8> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return data.to_vec();
    }

    const ICC_SIG: &[u8] = b"ICC_PROFILE\0";
    let mut out = vec![0xFFu8, 0xD8];
    let mut i = 2usize;

    while i < data.len() {
        if data[i] != 0xFF {
            out.extend_from_slice(&data[i..]);
            break;
        }
        i += 1;
        while i < data.len() && data[i] == 0xFF {
            i += 1;
        }
        if i >= data.len() {
            break;
        }

        let marker = data[i];
        i += 1;

        if marker == 0xD8 {
            out.extend_from_slice(&[0xFF, 0xD8]);
            continue;
        }
        if marker == 0xD9 {
            out.extend_from_slice(&[0xFF, 0xD9]);
            break;
        }
        if (0xD0..=0xD7).contains(&marker) {
            out.push(0xFF);
            out.push(marker);
            continue;
        }
        if marker == 0xDA {
            out.push(0xFF);
            out.push(0xDA);
            out.extend_from_slice(&data[i..]);
            break;
        }

        if i + 2 > data.len() {
            break;
        }
        let seg_len = ((data[i] as usize) << 8) | data[i + 1] as usize;
        if i + seg_len > data.len() {
            break;
        }

        if marker == 0xE2 && seg_len >= 2 + ICC_SIG.len() {
            let payload = &data[i + 2..i + seg_len];
            if payload.starts_with(ICC_SIG) {
                i += seg_len;
                continue;
            }
        }

        out.push(0xFF);
        out.push(marker);
        out.extend_from_slice(&data[i..i + seg_len]);
        i += seg_len;
    }

    out
}

pub fn is_image(name: &str) -> bool {
    let lower = name.to_lowercase();
    [".jpg", ".jpeg", ".png", ".webp", ".avif"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

/// Minimal scheme+host parse (no `url` crate — a few lines cover .NET's
/// `Uri.TryCreate(..., UriKind.Absolute, ...)` usage here).
pub fn root_domain_referer(url: &str) -> String {
    let Some(scheme_end) = url.find("://") else {
        return String::new();
    };
    let scheme = &url[..scheme_end];
    if scheme.is_empty()
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return String::new();
    }

    let rest = &url[scheme_end + 3..];
    let host_end = rest.find(['/', '?', '#', ':']).unwrap_or(rest.len());
    let host = &rest[..host_end];
    if host.is_empty() {
        return String::new();
    }

    let parts: Vec<&str> = host.split('.').collect();
    let root = if parts.len() >= 2 {
        format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1])
    } else {
        host.to_string()
    };
    format!("{scheme}://{root}")
}

/// Content-type by file extension — port of `ExtFromPath` (cover-serving,
/// not the 5-way `detect_content_type` used for downloaded pages).
pub fn content_type_from_path(path: &str) -> &'static str {
    match path.rsplit('.').next().map(|s| s.to_lowercase()) {
        Some(ext) if ext == "webp" => "image/webp",
        Some(ext) if ext == "png" => "image/png",
        _ => "image/jpeg",
    }
}

// ── file-backed chapter pages ────────────────────────────────────────────

/// Reads page `page` (0-indexed) from a downloaded chapter at `path` — a
/// `.cbz`/`.zip` archive or a plain directory of image files, sorted by
/// name either way. `None` on any failure (missing path, out of range,
/// corrupt archive) — caller falls back to a fresh source fetch.
pub async fn get_chapter_page(path: &str, page: usize) -> Option<Vec<u8>> {
    let ext = path.rsplit('.').next().map(|s| s.to_lowercase());
    if matches!(ext.as_deref(), Some("cbz") | Some("zip")) {
        let path = path.to_string();
        tokio::task::spawn_blocking(move || get_zip_page(&path, page))
            .await
            .ok()
            .flatten()
    } else {
        get_dir_page(path, page).await
    }
}

fn get_zip_page(path: &str, page: usize) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let mut names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .filter(|n| is_image(n))
        .collect();
    names.sort();
    let name = names.get(page)?;
    let mut entry = zip.by_name(name).ok()?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut buf).ok()?;
    Some(buf)
}

async fn get_dir_page(dir_path: &str, page: usize) -> Option<Vec<u8>> {
    let mut entries = tokio::fs::read_dir(dir_path).await.ok()?;
    let mut files = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if is_image(&name) {
            files.push(entry.path());
        }
    }
    files.sort();
    let path = files.get(page)?;
    tokio::fs::read(path).await.ok()
}

// ── DB access ─────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
pub struct ChapterLocalInfo {
    pub local_path: Option<String>,
    pub downloaded: bool,
}

pub async fn chapter_local_info(
    pool: &SqlitePool,
    chapter_id: &str,
) -> sqlx::Result<Option<ChapterLocalInfo>> {
    sqlx::query_as("SELECT local_path, downloaded FROM chapters WHERE id = ?")
        .bind(chapter_id)
        .fetch_optional(pool)
        .await
}

pub async fn reset_chapter_local(pool: &SqlitePool, chapter_id: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE chapters SET downloaded = 0, local_path = NULL WHERE id = ?")
        .bind(chapter_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// `(source_id, base_url)` for a chapter's sources, lowest
/// `external_sources.priority` first (unmatched sources sort last,
/// `base_url = NULL`, and are skipped by the caller) — port of
/// `ServePage`'s left-join source query.
pub async fn chapter_source_links(
    pool: &SqlitePool,
    chapter_id: &str,
) -> sqlx::Result<Vec<(String, Option<String>)>> {
    sqlx::query_as(
        "SELECT cs.source_id, es.base_url, COALESCE(es.priority, 100) AS priority \
         FROM chapter_sources cs LEFT JOIN external_sources es ON es.source_key = cs.source \
         WHERE cs.chapter_id = ? ORDER BY priority",
    )
    .bind(chapter_id)
    .fetch_all(pool)
    .await
    .map(|rows: Vec<(String, Option<String>, i64)>| {
        rows.into_iter().map(|(sid, base, _)| (sid, base)).collect()
    })
}

pub async fn title_cover_info(
    pool: &SqlitePool,
    title_id: &str,
) -> sqlx::Result<Option<(Option<String>, String)>> {
    sqlx::query_as("SELECT cover_url, title FROM titles WHERE id = ?")
        .bind(title_id)
        .fetch_optional(pool)
        .await
}

pub async fn clear_title_cover(pool: &SqlitePool, title_id: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE titles SET cover_url = NULL WHERE id = ?")
        .bind(title_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_title_cover(
    pool: &SqlitePool,
    title_id: &str,
    cover_url: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE titles SET cover_url = ? WHERE id = ?")
        .bind(cover_url)
        .bind(title_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn title_meta_cover_cdn(pool: &SqlitePool, key: &str) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT cover_cdn_url FROM title_meta WHERE title_key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map(|v| v.flatten())
}

pub async fn title_meta_cover_row(
    pool: &SqlitePool,
    key: &str,
) -> sqlx::Result<Option<(Option<String>, Option<String>)>> {
    sqlx::query_as("SELECT cover_local_path, cover_cdn_url FROM title_meta WHERE title_key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
}

pub async fn clear_title_meta_local_path(pool: &SqlitePool, key: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE title_meta SET cover_local_path = NULL WHERE title_key = ?")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_content_type_jpeg() {
        assert_eq!(
            detect_content_type(&[0xFF, 0xD8, 0x00, 0x00]),
            Some("image/jpeg")
        );
    }

    #[test]
    fn detect_content_type_png() {
        assert_eq!(
            detect_content_type(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
            Some("image/png")
        );
    }

    #[test]
    fn detect_content_type_webp() {
        let data = [
            0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50,
        ];
        assert_eq!(detect_content_type(&data), Some("image/webp"));
    }

    #[test]
    fn detect_content_type_gif() {
        assert_eq!(detect_content_type(b"GIF89a\x00\x00"), Some("image/gif"));
    }

    #[test]
    fn detect_content_type_avif() {
        let data = [
            0x00, 0x00, 0x00, 0x1C, 0x66, 0x74, 0x79, 0x70, 0x61, 0x76, 0x69, 0x66,
        ];
        assert_eq!(detect_content_type(&data), Some("image/avif"));
    }

    #[test]
    fn detect_content_type_too_short_returns_none() {
        assert_eq!(detect_content_type(&[0xFF, 0xD8, 0x00]), None);
    }

    #[test]
    fn detect_content_type_empty_returns_none() {
        assert_eq!(detect_content_type(&[]), None);
    }

    #[test]
    fn detect_content_type_unknown_returns_none() {
        assert_eq!(detect_content_type(&[0x00, 0x01, 0x02, 0x03]), None);
    }

    #[test]
    fn strip_jpeg_icc_non_jpeg_returned_unchanged() {
        let png = [0x89u8, 0x50, 0x4E, 0x47, 0x00];
        assert_eq!(strip_jpeg_icc(&png), png);
    }

    #[test]
    fn strip_jpeg_icc_too_short_returned_unchanged() {
        assert_eq!(strip_jpeg_icc(&[0xFF]), vec![0xFF]);
        assert_eq!(strip_jpeg_icc(&[]), Vec::<u8>::new());
    }

    #[test]
    fn strip_jpeg_icc_without_icc_preserves_soi() {
        let jpeg = [0xFFu8, 0xD8, 0xFF, 0xD9];
        let result = strip_jpeg_icc(&jpeg);
        assert_eq!(&result[..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn strip_jpeg_icc_strips_icc_segment() {
        let icc_payload = b"ICC_PROFILE\0fake icc data";
        let seg_len = (2 + icc_payload.len()) as u16;
        let mut jpeg = vec![0xFFu8, 0xD8];
        jpeg.extend_from_slice(&[0xFF, 0xE2]);
        jpeg.push((seg_len >> 8) as u8);
        jpeg.push((seg_len & 0xFF) as u8);
        jpeg.extend_from_slice(icc_payload);
        jpeg.extend_from_slice(&[0xFF, 0xD9]);

        let result = strip_jpeg_icc(&jpeg);
        assert!(!contains_sequence(&result, b"ICC_PROFILE\0"));
        assert_eq!(&result[..2], &[0xFF, 0xD8]);
    }

    fn contains_sequence(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn is_image_known_extensions() {
        for name in [
            "page.jpg",
            "page.jpeg",
            "page.PNG",
            "page.WEBP",
            "page.avif",
        ] {
            assert!(is_image(name), "{name} should be an image");
        }
    }

    #[test]
    fn is_image_non_image() {
        for name in ["page.txt", "page.cbz", "page", "page.html"] {
            assert!(!is_image(name), "{name} should not be an image");
        }
    }

    #[test]
    fn root_domain_referer_subdomain_extracts_root() {
        assert_eq!(
            root_domain_referer("https://cdn.mangapill.com/img/page.jpg"),
            "https://mangapill.com"
        );
    }

    #[test]
    fn root_domain_referer_apex_domain_returns_self() {
        assert_eq!(
            root_domain_referer("https://mangadex.org/chapter/abc"),
            "https://mangadex.org"
        );
    }

    #[test]
    fn root_domain_referer_invalid_url_returns_empty() {
        assert_eq!(root_domain_referer("not-a-url"), "");
    }

    #[test]
    fn root_domain_referer_empty_returns_empty() {
        assert_eq!(root_domain_referer(""), "");
    }

    #[test]
    fn root_domain_referer_preserves_scheme() {
        assert_eq!(
            root_domain_referer("http://sub.example.com/path"),
            "http://example.com"
        );
    }

    #[test]
    fn normalize_title_matches_discover_module() {
        // Same cache-key contract — see module doc.
        assert_eq!(crate::discover::normalize_title("One Piece"), "one piece");
        assert_eq!(
            crate::discover::normalize_title("I Shall Seal the Heavens!"),
            "i shall seal the heavens"
        );
    }

    #[tokio::test]
    async fn get_chapter_page_missing_path_returns_none() {
        assert!(get_chapter_page("/nonexistent/path", 0).await.is_none());
    }

    #[tokio::test]
    async fn get_chapter_page_dir_reads_correct_file() {
        let dir = std::env::temp_dir().join(format!("arrgh-media-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("01.jpg"), [0xFF, 0xD8, 0x01])
            .await
            .unwrap();
        tokio::fs::write(dir.join("02.jpg"), [0xFF, 0xD8, 0x02])
            .await
            .unwrap();

        let page0 = get_chapter_page(dir.to_str().unwrap(), 0).await.unwrap();
        let page1 = get_chapter_page(dir.to_str().unwrap(), 1).await.unwrap();
        assert_eq!(page0, vec![0xFF, 0xD8, 0x01]);
        assert_eq!(page1, vec![0xFF, 0xD8, 0x02]);

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn get_chapter_page_dir_out_of_range_returns_none() {
        let dir = std::env::temp_dir().join(format!("arrgh-media-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("01.jpg"), [0xFF, 0xD8])
            .await
            .unwrap();

        assert!(get_chapter_page(dir.to_str().unwrap(), 5).await.is_none());

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn get_chapter_page_zip_extracts_correct_entry() {
        let path =
            std::env::temp_dir().join(format!("arrgh-media-test-{}.cbz", uuid::Uuid::new_v4()));
        {
            let file = std::fs::File::create(&path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
            zip.start_file("01.jpg", options).unwrap();
            std::io::Write::write_all(&mut zip, &[0xFF, 0xD8, 0x01]).unwrap();
            zip.start_file("02.jpg", options).unwrap();
            std::io::Write::write_all(&mut zip, &[0xFF, 0xD8, 0x02]).unwrap();
            zip.finish().unwrap();
        }

        let page0 = get_chapter_page(path.to_str().unwrap(), 0).await.unwrap();
        assert_eq!(page0, vec![0xFF, 0xD8, 0x01]);

        std::fs::remove_file(&path).unwrap();
    }
}
