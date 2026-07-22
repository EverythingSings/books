//! Best-effort book cover fetcher backed by Open Library, with on-disk caching
//! to `public/covers/`.
//!
//! Strategy:
//! 1. If `public/covers/<slug>.jpg` already exists, return its public URL — no network call.
//! 2. Else query multiple Open Library results by title and author, then retry by title.
//! 3. Fetch the first matching real cover and write it to disk.
//! 4. If lookup fails, leave the review uncovered for this build and retry on a future build.
//!
//! All cover fetching is opt-in via `BOOKS_FETCH_COVERS=1` env var so local dev builds
//! don't unexpectedly access the network. Requests are throttled to Open Library's
//! identified-client rate limit.

use serde::Deserialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

const USER_AGENT: &str =
    "books.everythingsings.art cover-fetcher (contact: EverythingSings@primal.net)";

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    docs: Vec<SearchDoc>,
}

#[derive(Debug, Deserialize)]
struct SearchDoc {
    #[serde(default)]
    cover_i: Option<i64>,
    #[serde(default)]
    title: String,
    #[serde(default)]
    author_name: Vec<String>,
}

pub struct CoverCache {
    covers_dir: PathBuf,
    fetch_enabled: bool,
}

impl CoverCache {
    pub fn new(covers_dir: PathBuf) -> Self {
        let fetch_enabled = std::env::var("BOOKS_FETCH_COVERS")
            .map(|v| v == "1")
            .unwrap_or(false);
        Self {
            covers_dir,
            fetch_enabled,
        }
    }

    /// Returns the public URL path (e.g. `/covers/sapiens.jpg`) if a cover exists or
    /// could be fetched; otherwise `None`.
    pub fn cover_path(&self, slug: &str, title: &str, author: &str) -> Option<String> {
        let jpg = self.covers_dir.join(format!("{slug}.jpg"));

        if jpg.exists() {
            return Some(format!("/covers/{slug}.jpg"));
        }
        if !self.fetch_enabled {
            return None;
        }

        fs::create_dir_all(&self.covers_dir).ok()?;

        match try_fetch(title, author) {
            Ok(Some(bytes)) => {
                if fs::write(&jpg, &bytes).is_ok() {
                    println!(
                        "  cover: fetched /covers/{slug}.jpg ({} bytes)",
                        bytes.len()
                    );
                    Some(format!("/covers/{slug}.jpg"))
                } else {
                    None
                }
            }
            Ok(None) => None,
            Err(e) => {
                eprintln!("  cover: error for {slug}: {e}");
                None
            }
        }
    }
}

fn try_fetch(title: &str, author: &str) -> Result<Option<Vec<u8>>, String> {
    let q_title = urlencode(title);
    let q_author = urlencode(author);
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .user_agent(USER_AGENT)
        .build();

    let mut searches = Vec::with_capacity(2);
    if author.is_empty() {
        searches.push(format!(
            "https://openlibrary.org/search.json?title={q_title}&fields=cover_i,title,author_name&limit=20"
        ));
    } else {
        searches.push(format!(
            "https://openlibrary.org/search.json?title={q_title}&author={q_author}&fields=cover_i,title,author_name&limit=20"
        ));
        // Author metadata varies across editions. A title-only retry catches those
        // records, but requires a title match before accepting the cover.
        searches.push(format!(
            "https://openlibrary.org/search.json?title={q_title}&fields=cover_i,title,author_name&limit=20"
        ));
    }

    let mut cover_id = None;
    for url in searches {
        // Identified clients are allowed three requests per second. Stay just below
        // that limit even when many old reviews are backfilled in one run.
        std::thread::sleep(Duration::from_millis(350));
        let resp = agent.get(&url).call().map_err(|e| e.to_string())?;
        let body = resp.into_string().map_err(|e| e.to_string())?;
        let parsed: SearchResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
        cover_id = parsed
            .docs
            .iter()
            // Search ranking can include unrelated older books that merely contain
            // the requested phrase. Never trust rank alone, even with an author.
            .filter(|doc| {
                title_matches(title, &doc.title)
                    && (author.is_empty() || author_matches(author, &doc.author_name))
            })
            .find_map(|doc| doc.cover_i);
        if cover_id.is_some() {
            break;
        }
    }

    let Some(cover_i) = cover_id else {
        return Ok(None);
    };
    let cover_url = format!("https://covers.openlibrary.org/b/id/{cover_i}-L.jpg");
    let mut img_resp = agent
        .get(&cover_url)
        .call()
        .map_err(|e| e.to_string())?
        .into_reader();
    let mut bytes = Vec::with_capacity(64 * 1024);
    img_resp
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    // OpenLibrary returns a tiny 1x1 placeholder when there's no real cover.
    if bytes.len() < 1024 {
        return Ok(None);
    }
    Ok(Some(bytes))
}

fn title_matches(requested: &str, candidate: &str) -> bool {
    let requested = normalize_title(requested);
    let candidate = normalize_title(candidate);
    requested == candidate
        || candidate
            .strip_prefix(&requested)
            .is_some_and(|suffix| suffix.starts_with(' '))
        || requested
            .strip_prefix(&candidate)
            .is_some_and(|suffix| suffix.starts_with(' '))
}

fn normalize_title(title: &str) -> String {
    let mut normalized = String::with_capacity(title.len());
    let mut needs_space = false;
    for c in title.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            if needs_space && !normalized.is_empty() {
                normalized.push(' ');
            }
            normalized.push(c);
            needs_space = false;
        } else {
            needs_space = true;
        }
    }
    normalized
}

fn author_matches(requested: &str, candidates: &[String]) -> bool {
    let requested = author_tokens(requested);
    candidates.iter().any(|candidate| {
        let candidate = author_tokens(candidate);
        requested == candidate
            || (!requested.is_empty()
                && !candidate.is_empty()
                && (requested.last() == candidate.last()
                    || (candidate.len() == 1 && requested.contains(&candidate[0]))
                    || (requested.len() == 1 && candidate.contains(&requested[0]))
                    || (requested.iter().all(|token| candidate.contains(token))
                        && candidate.iter().all(|token| requested.contains(token)))))
    })
}

fn author_tokens(author: &str) -> Vec<String> {
    normalize_title(author)
        .split_whitespace()
        .filter(|token| !matches!(*token, "and" | "with" | "by"))
        .map(str::to_owned)
        .collect()
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
        assert_eq!(
            urlencode("Sapiens: A Brief History"),
            "Sapiens%3A+A+Brief+History"
        );
    }

    #[test]
    fn title_match_accepts_subtitles_but_not_prefix_words() {
        assert!(title_matches(
            "AI 2041",
            "AI 2041: Ten Visions for Our Future"
        ));
        assert!(title_matches("Dune: Messiah", "Dune Messiah"));
        assert!(!title_matches("Build", "Building a Second Brain"));
        assert!(!title_matches("Breath", "Breathless"));
        assert!(!title_matches(
            "The Romance of Reality",
            "Historical Tales: The Romance of Reality"
        ));
    }

    #[test]
    fn author_match_handles_aliases_and_name_order_without_accepting_another_author() {
        assert!(author_matches("Cixin Liu", &["Liu Cixin".to_owned()]));
        assert!(author_matches("Bobby Hall (Logic)", &["Logic".to_owned()]));
        assert!(!author_matches("Haris Ward", &["Chaim Potok".to_owned()]));
    }
}
