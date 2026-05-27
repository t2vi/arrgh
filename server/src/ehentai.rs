// Two-step E-Hentai search:
// 1. Scrape https://e-hentai.org/?f_search=QUERY+language:english for gallery ID/token pairs
// 2. POST https://api.e-hentai.org/api.php gdata to get full metadata
// Group results by parody tag; originals each get their own result card.

use std::collections::HashMap;
use std::sync::OnceLock;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

pub struct EHentaiClient<'a> {
    http: &'a reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct EhSeries {
    pub series_id: String,     // "ehentai:parody:jk-tsuma" or "ehentai:g:12345"
    pub title: String,
    pub cover_url: Option<String>,
    pub author: Option<String>,
    pub tags: String,          // comma-separated, always includes "hentai"
    pub gallery_count: usize,
}

impl<'a> EHentaiClient<'a> {
    pub fn new(http: &'a reqwest::Client) -> Self {
        Self { http }
    }

    pub async fn search(&self, query: &str) -> anyhow::Result<Vec<EhSeries>> {
        let pairs = self.scrape_search(query).await?;
        if pairs.is_empty() {
            return Ok(Vec::new());
        }
        let galleries = self.fetch_gdata(&pairs).await?;
        Ok(group_by_parody(galleries))
    }

    async fn scrape_search(&self, query: &str) -> anyhow::Result<Vec<(u64, String)>> {
        let search_term = format!("{} language:english", query);
        let url = format!(
            "https://e-hentai.org/?f_search={}",
            urlencoding::encode(&search_term)
        );

        let html = self.http.get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Ok(extract_gallery_pairs(&html))
    }

    async fn fetch_gdata(&self, pairs: &[(u64, String)]) -> anyhow::Result<Vec<GalleryMeta>> {
        #[derive(Serialize)]
        struct GdataReq<'b> {
            method: &'static str,
            gidlist: &'b [(u64, String)],
            namespace: u8,
        }

        #[derive(Deserialize)]
        struct GdataResp {
            gmetadata: Vec<GalleryMeta>,
        }

        let body = GdataReq { method: "gdata", gidlist: pairs, namespace: 1 };

        let resp: GdataResp = self.http
            .post("https://api.e-hentai.org/api.php")
            .header("User-Agent", "Mozilla/5.0")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp.gmetadata)
    }
}

#[derive(Deserialize, Debug)]
struct GalleryMeta {
    gid: u64,
    title: String,
    #[serde(default)]
    thumb: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

fn extract_gallery_pairs(html: &str) -> Vec<(u64, String)> {
    static SEL: OnceLock<Selector> = OnceLock::new();
    let sel = SEL.get_or_init(|| Selector::parse("a").unwrap());

    let doc = Html::parse_document(html);
    let mut pairs = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for a in doc.select(sel) {
        if let Some(href) = a.value().attr("href") {
            if let Some(pair) = parse_gallery_url(href) {
                if seen.insert(pair.0) {
                    pairs.push(pair);
                }
            }
        }
        if pairs.len() >= 25 {
            break;
        }
    }

    pairs
}

fn parse_gallery_url(url: &str) -> Option<(u64, String)> {
    // Matches https://e-hentai.org/g/{id}/{token}[/]
    let url = url.trim_end_matches('/');
    let pos = url.find("/g/")?;
    let rest = &url[pos + 3..];
    let mut parts = rest.splitn(2, '/');
    let id: u64 = parts.next()?.parse().ok()?;
    let token = parts.next()?.to_string();
    if id == 0 || token.is_empty() {
        return None;
    }
    Some((id, token))
}

fn group_by_parody(galleries: Vec<GalleryMeta>) -> Vec<EhSeries> {
    let mut parody_order: Vec<String> = Vec::new();
    let mut by_parody: HashMap<String, Vec<GalleryMeta>> = HashMap::new();
    let mut originals: Vec<GalleryMeta> = Vec::new();

    for g in galleries {
        let parody = g.tags.iter()
            .find(|t| t.starts_with("parody:"))
            .map(|t| t.trim_start_matches("parody:").to_string());

        match parody {
            Some(p) => {
                if !by_parody.contains_key(&p) {
                    parody_order.push(p.clone());
                }
                by_parody.entry(p).or_default().push(g);
            }
            None => originals.push(g),
        }
    }

    let mut results = Vec::new();

    for key in &parody_order {
        let group = by_parody.remove(key).unwrap_or_default();
        let rep = &group[0];
        let series_id = format!("ehentai:parody:{}", key.replace(' ', "-").to_lowercase());
        results.push(EhSeries {
            series_id,
            title: prettify_tag(key),
            cover_url: rep.thumb.clone(),
            author: extract_artist(&rep.tags),
            tags: build_tags(&rep.tags),
            gallery_count: group.len(),
        });
    }

    for g in originals {
        results.push(EhSeries {
            series_id: format!("ehentai:g:{}", g.gid),
            title: g.title.clone(),
            cover_url: g.thumb.clone(),
            author: extract_artist(&g.tags),
            tags: build_tags(&g.tags),
            gallery_count: 1,
        });
    }

    results
}

fn prettify_tag(tag: &str) -> String {
    tag.replace('-', " ")
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_artist(tags: &[String]) -> Option<String> {
    tags.iter()
        .find(|t| t.starts_with("artist:"))
        .map(|t| prettify_tag(t.trim_start_matches("artist:")))
}

fn build_tags(raw: &[String]) -> String {
    // Include non-namespaced tags plus parody/character values; always prepend "hentai"
    let mut tags: Vec<String> = std::iter::once("hentai".to_string())
        .chain(raw.iter().filter_map(|t| {
            if t == "hentai" {
                None // avoid duplicate
            } else if let Some(rest) = t.strip_prefix("parody:") {
                Some(rest.to_string())
            } else if t.contains(':') {
                None // skip other namespaced tags
            } else {
                Some(t.clone())
            }
        }))
        .collect();

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    tags.retain(|t| seen.insert(t.clone()));

    tags.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gallery_url_valid() {
        let r = parse_gallery_url("https://e-hentai.org/g/12345/abc123def/");
        assert_eq!(r, Some((12345, "abc123def".to_string())));
    }

    #[test]
    fn parse_gallery_url_no_trailing_slash() {
        let r = parse_gallery_url("https://e-hentai.org/g/99/xyz");
        assert_eq!(r, Some((99, "xyz".to_string())));
    }

    #[test]
    fn parse_gallery_url_invalid() {
        assert!(parse_gallery_url("https://e-hentai.org/").is_none());
        assert!(parse_gallery_url("https://google.com/g/1/abc").is_some()); // any /g/ path
    }

    #[test]
    fn prettify_tag_hyphenated() {
        assert_eq!(prettify_tag("jk-tsuma"), "Jk Tsuma");
    }

    #[test]
    fn build_tags_always_has_hentai() {
        let raw = vec!["parody:naruto".to_string(), "big-breasts".to_string()];
        let tags = build_tags(&raw);
        assert!(tags.starts_with("hentai"));
        assert!(tags.contains("naruto"));
    }

    #[test]
    fn group_originals_when_no_parody() {
        let galleries = vec![
            GalleryMeta { gid: 1, title: "Work A".into(), thumb: None, tags: vec![] },
            GalleryMeta { gid: 2, title: "Work B".into(), thumb: None, tags: vec![] },
        ];
        let result = group_by_parody(galleries);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].series_id, "ehentai:g:1");
    }

    #[test]
    fn group_parody_merges_galleries() {
        let galleries = vec![
            GalleryMeta { gid: 1, title: "A".into(), thumb: None, tags: vec!["parody:naruto".into()] },
            GalleryMeta { gid: 2, title: "B".into(), thumb: None, tags: vec!["parody:naruto".into()] },
            GalleryMeta { gid: 3, title: "C".into(), thumb: None, tags: vec![] },
        ];
        let result = group_by_parody(galleries);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].series_id, "ehentai:parody:naruto");
        assert_eq!(result[0].gallery_count, 2);
        assert_eq!(result[1].series_id, "ehentai:g:3");
    }
}
