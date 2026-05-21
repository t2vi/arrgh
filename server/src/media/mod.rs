use anyhow::Result;
use std::path::Path;
use tokio::fs;

pub async fn get_chapter_page(chapter_path: &str, page: usize) -> Result<Vec<u8>> {
    let path = Path::new(chapter_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let data = match ext.to_lowercase().as_str() {
        "cbz" | "zip" => extract_zip_page(chapter_path, page).await,
        _ => read_dir_page(chapter_path, page).await,
    }?;

    Ok(strip_jpeg_icc(data))
}

/// Strip embedded ICC_PROFILE segments from JPEG data without re-encoding.
/// Non-JPEG data (PNG, WebP) is returned unchanged.
pub fn strip_jpeg_icc(data: Vec<u8>) -> Vec<u8> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return data;
    }

    let mut out = Vec::with_capacity(data.len());
    out.push(0xFF);
    out.push(0xD8); // SOI

    let mut i = 2usize;
    while i < data.len() {
        // Advance to next 0xFF marker byte
        if data[i] != 0xFF {
            out.extend_from_slice(&data[i..]);
            break;
        }
        i += 1;
        // Skip padding 0xFF bytes
        while i < data.len() && data[i] == 0xFF {
            i += 1;
        }
        if i >= data.len() {
            break;
        }
        let marker = data[i];
        i += 1;

        match marker {
            0xD8 => { out.extend_from_slice(b"\xFF\xD8"); }
            0xD9 => { out.extend_from_slice(b"\xFF\xD9"); break; }
            // RST markers have no payload
            0xD0..=0xD7 => { out.push(0xFF); out.push(marker); }
            // SOS: entropy-coded data follows — copy rest verbatim
            0xDA => {
                out.push(0xFF);
                out.push(0xDA);
                out.extend_from_slice(&data[i..]);
                break;
            }
            _ => {
                if i + 2 > data.len() { break; }
                let seg_len = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
                if i + seg_len > data.len() { break; }
                let payload = &data[i + 2..i + seg_len];

                // APP2 ICC_PROFILE — drop this segment
                if marker == 0xE2 && payload.starts_with(b"ICC_PROFILE\x00") {
                    i += seg_len;
                    continue;
                }

                out.push(0xFF);
                out.push(marker);
                out.extend_from_slice(&data[i..i + seg_len]);
                i += seg_len;
            }
        }
    }

    out
}

async fn extract_zip_page(archive_path: &str, page: usize) -> Result<Vec<u8>> {
    let path = archive_path.to_string();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut names: Vec<String> = (0..archive.len())
            .filter_map(|i| {
                archive.by_index(i).ok().and_then(|f| {
                    let name = f.name().to_string();
                    if is_image(&name) { Some(name) } else { None }
                })
            })
            .collect();
        names.sort();

        let name = names.get(page).ok_or_else(|| anyhow::anyhow!("page out of range"))?.clone();
        let mut entry = archive.by_name(&name)?;
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut buf)?;
        Ok(buf)
    })
    .await?
}

async fn read_dir_page(dir_path: &str, page: usize) -> Result<Vec<u8>> {
    let mut read_dir = fs::read_dir(dir_path).await?;
    let mut entries = Vec::new();
    while let Some(entry) = read_dir.next_entry().await? {
        if is_image(&entry.file_name().to_string_lossy()) {
            entries.push(entry);
        }
    }
    entries.sort_by_key(|e| e.file_name());

    let entry = entries.get(page).ok_or_else(|| anyhow::anyhow!("page out of range"))?;
    let data = fs::read(entry.path()).await?;
    Ok(data)
}

fn is_image(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp")
        || lower.ends_with(".avif")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_image ──────────────────────────────────────────────────────────────

    #[test]
    fn is_image_accepts_known_extensions() {
        for name in ["page.jpg", "page.jpeg", "page.PNG", "page.WEBP", "page.avif"] {
            assert!(is_image(name), "{name} should be recognised as an image");
        }
    }

    #[test]
    fn is_image_rejects_non_image() {
        for name in ["page.txt", "page.cbz", "page", "page.html"] {
            assert!(!is_image(name), "{name} should not be recognised as an image");
        }
    }

    // ── strip_jpeg_icc ────────────────────────────────────────────────────────

    #[test]
    fn non_jpeg_returned_unchanged() {
        let png = b"\x89PNG\r\n\x1a\nsome data".to_vec();
        assert_eq!(strip_jpeg_icc(png.clone()), png);
    }

    #[test]
    fn too_short_returned_unchanged() {
        assert_eq!(strip_jpeg_icc(vec![0xFF]), vec![0xFF]);
        assert_eq!(strip_jpeg_icc(vec![]), Vec::<u8>::new());
    }

    #[test]
    fn jpeg_without_icc_passes_through() {
        // Minimal valid JPEG: SOI + EOI
        let jpeg = vec![0xFF, 0xD8, 0xFF, 0xD9];
        let result = strip_jpeg_icc(jpeg);
        assert_eq!(&result[..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn jpeg_with_icc_segment_stripped() {
        // Build a minimal JPEG with one APP2 ICC_PROFILE segment
        let mut jpeg = vec![0xFF, 0xD8]; // SOI
        // APP2 marker (0xFF 0xE2) + length (2 + payload) + ICC_PROFILE\0 header
        let icc_payload = b"ICC_PROFILE\x00fake icc data";
        let seg_len = (2 + icc_payload.len()) as u16;
        jpeg.extend_from_slice(&[0xFF, 0xE2]);
        jpeg.extend_from_slice(&seg_len.to_be_bytes());
        jpeg.extend_from_slice(icc_payload);
        jpeg.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let result = strip_jpeg_icc(jpeg);

        // ICC segment must be gone
        assert!(!result.windows(12).any(|w| w == b"ICC_PROFILE\x00"),
            "ICC_PROFILE segment should be stripped");
        // SOI must be preserved
        assert_eq!(&result[..2], &[0xFF, 0xD8]);
    }
}
