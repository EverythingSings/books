//! Book cover fetcher backed by OpenLibrary, with on-disk caching to `public/covers/`.
//!
//! Strategy:
//! 1. If `public/covers/<slug>.jpg` already exists, return its public URL — no network call.
//! 2. Else query `openlibrary.org/search.json?title=...&author=...&limit=1`.
//! 3. If the search result has `cover_i`, fetch `covers.openlibrary.org/b/id/<id>-L.jpg`
//!    and write to disk.
//! 4. On any failure, write a `<slug>.miss` marker so we don't retry every build.
//!
//! All cover fetching is opt-in via `BOOKS_FETCH_COVERS=1` env var so local dev builds
//! don't slam the API.

use serde::Deserialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

const USER_AGENT: &str = "books.everythingsings.art cover-fetcher (contact: EverythingSings@primal.net)";

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    docs: Vec<SearchDoc>,
}

#[derive(Debug, Deserialize)]
struct SearchDoc {
    #[serde(default)]
    cover_i: Option<i64>,
}

pub struct CoverCache {
    covers_dir: PathBuf,
    fetch_enabled: bool,
}

impl CoverCache {
    pub fn new(covers_dir: PathBuf) -> Self {
        let fetch_enabled = std::env::var("BOOKS_FETCH_COVERS").map(|v| v == "1").unwrap_or(false);
        Self { covers_dir, fetch_enabled }
    }

    /// Returns the public URL path (e.g. `/covers/sapiens.jpg`) if a cover exists or
    /// could be fetched; otherwise `None`.
    pub fn cover_path(&self, slug: &str, title: &str, author: &str) -> Option<String> {
        let jpg = self.covers_dir.join(format!("{slug}.jpg"));
        let miss = self.covers_dir.join(format!("{slug}.miss"));

        if jpg.exists() {
            return Some(format!("/covers/{slug}.jpg"));
        }
        if miss.exists() {
            return None;
        }
        if !self.fetch_enabled {
            return None;
        }

        fs::create_dir_all(&self.covers_dir).ok()?;

        match try_fetch(title, author) {
            Ok(Some(bytes)) => {
                if fs::write(&jpg, &bytes).is_ok() {
                    println!("  cover: fetched /covers/{slug}.jpg ({} bytes)", bytes.len());
                    Some(format!("/covers/{slug}.jpg"))
                } else {
                    None
                }
            }
            Ok(None) => {
                let _ = fs::write(&miss, "");
                None
            }
            Err(e) => {
                eprintln!("  cover: error for {slug}: {e}");
                let _ = fs::write(&miss, &e);
                None
            }
        }
    }
}

fn try_fetch(title: &str, author: &str) -> Result<Option<Vec<u8>>, String> {
    let q_title = urlencode(title);
    let q_author = urlencode(author);
    let url = if author.is_empty() {
        format!("https://openlibrary.org/search.json?title={q_title}&limit=1")
    } else {
        format!("https://openlibrary.org/search.json?title={q_title}&author={q_author}&limit=1")
    };
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .user_agent(USER_AGENT)
        .build();
    let resp = agent.get(&url).call().map_err(|e| e.to_string())?;
    let body = resp.into_string().map_err(|e| e.to_string())?;
    let parsed: SearchResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let Some(doc) = parsed.docs.first() else { return Ok(None) };
    let Some(cover_i) = doc.cover_i else { return Ok(None) };
    let cover_url = format!("https://covers.openlibrary.org/b/id/{cover_i}-L.jpg");
    let mut img_resp = agent.get(&cover_url).call().map_err(|e| e.to_string())?.into_reader();
    let mut bytes = Vec::with_capacity(64 * 1024);
    img_resp.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    // OpenLibrary returns a tiny 1x1 placeholder when there's no real cover.
    if bytes.len() < 1024 {
        return Ok(None);
    }
    Ok(Some(bytes))
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else if c == ' ' {
            out.push('+');
        } else {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).as_bytes() {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

pub fn covers_dir_under(public_dir: &Path) -> PathBuf {
    public_dir.join("covers")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_encode_basic() {
        assert_eq!(urlencode("Hello World"), "Hello+World");
        assert_eq!(urlencode("Sapiens: A Brief History"), "Sapiens%3A+A+Brief+History");
    }
}
