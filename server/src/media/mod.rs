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
