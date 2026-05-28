//! Loads reviews from `reviews/*.md`. Each file has TOML frontmatter delimited
//! by `+++` fences followed by a markdown body.

use pulldown_cmark::{html, Options, Parser};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
struct Frontmatter {
    pub number: u32,
    pub title: String,
    #[serde(default)]
    pub author: String,
    pub date: String,
    #[serde(default, rename = "date_raw")]
    pub _date_raw: String,
    #[serde(default)]
    pub link: String,
}

#[derive(Debug, Clone)]
pub struct Review {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub date: String,        // YYYY-MM-DD
    pub date_display: String, // human-friendly, e.g. "January 9, 2019"
    pub link: String,
    pub slug: String,
    pub body_html: String,
    pub body_text: String, // plaintext for description/feed
}

pub fn load_all(dir: &Path) -> std::io::Result<Vec<Review>> {
    let mut out: Vec<Review> = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        if let Some(review) = parse_one(&path, &text) {
            out.push(review);
        } else {
            eprintln!("warning: skipping malformed review file: {}", path.display());
        }
    }
    // Sort by review number (oldest first; matches original chronological order).
    out.sort_by_key(|r| r.number);
    Ok(out)
}

fn parse_one(path: &Path, text: &str) -> Option<Review> {
    let (fm_str, body) = split_frontmatter(text)?;
    let fm: Frontmatter = toml::from_str(fm_str).ok()?;
    let slug = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-').to_string())
        .unwrap_or_default();
    let slug = if slug.is_empty() {
        slugify(&fm.title)
    } else {
        slug
    };

    let body_html = render_markdown(body);
    let body_text = strip_markdown(body);
    let date_display = format_date(&fm.date);

    Some(Review {
        number: fm.number,
        title: fm.title,
        author: fm.author,
        date: fm.date,
        date_display,
        link: fm.link,
        slug,
        body_html,
        body_text,
    })
}

fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("+++")?;
    let rest = rest.trim_start_matches('\n');
    let end = rest.find("\n+++")?;
    let fm = &rest[..end];
    let after = &rest[end + 4..]; // skip "\n+++"
    let body = after.trim_start_matches(['\n', '\r']);
    Some((fm, body))
}

fn render_markdown(md: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);
    let parser = Parser::new_ext(md, opts);
    let mut html_out = String::with_capacity(md.len() + md.len() / 4);
    html::push_html(&mut html_out, parser);
    html_out
}

fn strip_markdown(md: &str) -> String {
    // First pass: replace `[text](url)` with just `text`.
    let mut without_links = String::with_capacity(md.len());
    let chars: Vec<char> = md.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            // Find matching ']'
            if let Some(close) = chars[i + 1..].iter().position(|&c| c == ']') {
                let close = close + i + 1;
                // Check for following '('
                if chars.get(close + 1) == Some(&'(') {
                    // Find matching ')'
                    if let Some(rparen) = chars[close + 2..].iter().position(|&c| c == ')') {
                        let rparen = rparen + close + 2;
                        // Emit the link text only.
                        without_links.extend(&chars[i + 1..close]);
                        i = rparen + 1;
                        continue;
                    }
                }
            }
        }
        without_links.push(chars[i]);
        i += 1;
    }

    // Second pass: drop markdown punctuation we don't want in plaintext.
    let stripped: String = without_links
        .chars()
        .filter(|c| !matches!(*c, '*' | '_' | '`' | '#' | '>'))
        .collect();
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Today's date in the same human-readable form as review dates.
/// Uses UTC because the deploy runs on GitHub-hosted runners.
pub fn today_display() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format_date(&format!("{y:04}-{m:02}-{d:02}"))
}

/// Howard Hinnant's civil-from-days algorithm. Converts days since the
/// Unix epoch (1970-01-01) to a (year, month, day) Gregorian triple.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z / 146_097 } else { (z - 146_096) / 146_097 };
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn format_date(iso: &str) -> String {
    // Expect "YYYY-MM-DD" or "YYYY-MM-01" for month-only dates.
    let parts: Vec<&str> = iso.split('-').collect();
    if parts.len() != 3 {
        return iso.to_string();
    }
    let year = parts[0];
    let month: usize = parts[1].parse().unwrap_or(0);
    let day = parts[2];
    let name = match month {
        1 => "January", 2 => "February", 3 => "March", 4 => "April",
        5 => "May", 6 => "June", 7 => "July", 8 => "August",
        9 => "September", 10 => "October", 11 => "November", 12 => "December",
        _ => return iso.to_string(),
    };
    let d: u32 = day.parse().unwrap_or(0);
    if d == 0 {
        format!("{name} {year}")
    } else if day == "01" && iso.ends_with("-01") && parts[1] != "01" {
        // Heuristic: imports for "June 2020" got day=01; render without day
        // when raw source didn't include a day. Safer to always show the day.
        format!("{name} {d}, {year}")
    } else {
        format!("{name} {d}, {year}")
    }
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Convenience for tests / callers that want a `PathBuf` to a review.
pub fn review_output_path(slug: &str) -> PathBuf {
    PathBuf::from(slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_frontmatter_basic() {
        let text = "+++\nnumber = 1\ntitle = \"X\"\ndate = \"2020-01-01\"\n+++\n\nBody here.\n";
        let (fm, body) = split_frontmatter(text).unwrap();
        assert!(fm.contains("title"));
        assert_eq!(body, "Body here.\n");
    }

    #[test]
    fn render_markdown_basic() {
        let html = render_markdown("**hi**");
        assert!(html.contains("<strong>hi</strong>"));
    }

    #[test]
    fn strip_markdown_removes_links() {
        let stripped = strip_markdown("Read [this](https://x.com) now");
        assert_eq!(stripped, "Read this now");
    }

    #[test]
    fn format_date_full() {
        assert_eq!(format_date("2019-01-09"), "January 9, 2019");
        assert_eq!(format_date("2020-06-01"), "June 1, 2020");
    }
}
