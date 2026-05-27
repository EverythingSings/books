//! One-shot importer: parses the Obsidian-flavored `Book Reviews.md` master file
//! and emits one `reviews/NNN-slug.md` per entry with TOML frontmatter.
//!
//! Usage:
//!   cargo run --bin import -- <path/to/Book Reviews.md> [reviews/]
//!
//! Re-run idempotent: overwrites existing files.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: import <Book Reviews.md> [reviews/]");
        return ExitCode::from(2);
    }
    let src = PathBuf::from(&args[1]);
    let out_dir = PathBuf::from(args.get(2).map(String::as_str).unwrap_or("reviews"));

    let text = match fs::read_to_string(&src) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("read {}: {e}", src.display());
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = fs::create_dir_all(&out_dir) {
        eprintln!("mkdir {}: {e}", out_dir.display());
        return ExitCode::FAILURE;
    }

    let entries = split_entries(&text);
    let mut written = 0usize;
    for entry in &entries {
        let parsed = parse_entry(entry);
        let filename = format!("{:03}-{}.md", parsed.number, parsed.slug);
        let path = out_dir.join(&filename);
        let contents = render_review(&parsed);
        if let Err(e) = fs::write(&path, contents) {
            eprintln!("write {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        written += 1;
    }

    println!("imported {written} reviews into {}", out_dir.display());
    ExitCode::SUCCESS
}

#[derive(Debug)]
struct Entry {
    number: u32,
    title: String,
    author: String,
    date_iso: String,    // YYYY-MM-DD; defaults to "1970-01-01" if unparseable
    date_raw: String,    // original date string (preserve in frontmatter)
    link: String,        // may be empty
    body: String,        // markdown (Obsidian artefacts stripped)
    slug: String,
}

fn split_entries(text: &str) -> Vec<String> {
    // Headings look like `# 1: Title - Author` or `# 109: Universal Principles of Design`.
    // We split on lines that match `^# \d+: ` (no other heading uses that prefix).
    let mut entries: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut started = false;
    for line in text.lines() {
        if is_entry_heading(line) {
            if started {
                entries.push(current.join("\n"));
            }
            current.clear();
            current.push(line);
            started = true;
        } else if started {
            // stop accumulating at the "# Note to Reader" or any new top-level non-entry heading
            if line.starts_with("# ") && !is_entry_heading(line) {
                entries.push(current.join("\n"));
                current.clear();
                started = false;
                continue;
            }
            current.push(line);
        }
    }
    if started {
        entries.push(current.join("\n"));
    }
    entries
}

fn is_entry_heading(line: &str) -> bool {
    let rest = match line.strip_prefix("# ") {
        Some(r) => r,
        None => return false,
    };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return false;
    }
    rest[digits.len()..].starts_with(':')
}

fn parse_entry(entry: &str) -> Entry {
    let mut lines = entry.lines();
    let heading = lines.next().unwrap_or("");
    let (number, title_raw, author_in_title) = parse_heading(heading);

    // The remaining lines: a date line, a link line, then the review body.
    // Author may also appear as `**Author:** ...` (entry #109 style).
    let mut date_raw = String::new();
    let mut date_iso = String::new();
    let mut link = String::new();
    let mut author_field = String::new();
    let mut body_lines: Vec<&str> = Vec::new();
    let mut header_done = false;

    for line in lines {
        let trimmed = line.trim();
        if !header_done {
            if trimmed.is_empty() {
                continue;
            }
            // Skip Obsidian artefacts in the header area.
            if trimmed.starts_with("![[") || trimmed.starts_with('^') {
                continue;
            }
            // Author field?
            if let Some(rest) = trimmed.strip_prefix("**Author:**") {
                author_field = rest.trim().to_string();
                continue;
            }
            // Date field (entry #109 style)?
            if let Some(rest) = trimmed.strip_prefix("**Date:**") {
                let val = rest.trim();
                if let Some(iso) = parse_date(val) {
                    date_raw = val.to_string();
                    date_iso = iso;
                }
                continue;
            }
            // Star-rating line at the top — preserve as rating, don't push to body.
            if trimmed.chars().all(|c| "★☆".contains(c)) && !trimmed.is_empty() {
                continue;
            }
            // Date?
            if date_raw.is_empty() {
                if let Some(iso) = parse_date(trimmed) {
                    date_raw = trimmed.to_string();
                    date_iso = iso;
                    continue;
                }
            }
            // Link?
            if link.is_empty() {
                if let Some(url) = extract_url(trimmed) {
                    link = url;
                    continue;
                }
            }
            // Star-rating line — preserve in body so context isn't lost.
            // Anything else: header is done, treat from here as body.
            header_done = true;
        }
        body_lines.push(line);
    }

    let title = clean_text(&title_raw);
    let author = if !author_in_title.is_empty() {
        clean_text(&author_in_title)
    } else if !author_field.is_empty() {
        clean_text(&author_field)
    } else {
        String::new()
    };

    let body = clean_body(&body_lines.join("\n"));
    let slug = slugify(&title);

    Entry {
        number,
        title,
        author,
        date_iso: if date_iso.is_empty() {
            "1970-01-01".to_string()
        } else {
            date_iso
        },
        date_raw,
        link,
        body,
        slug,
    }
}

/// Parses `# 86: [[Elder Race]] - Adrian Tchaikovsky` -> (86, "Elder Race", "Adrian Tchaikovsky").
fn parse_heading(line: &str) -> (u32, String, String) {
    let rest = line.strip_prefix("# ").unwrap_or(line);
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let number = digits.parse::<u32>().unwrap_or(0);
    let after = rest[digits.len()..].trim_start_matches(':').trim();
    // Author separator: " - " (regular hyphen) or " – " (en-dash) or " — " (em-dash).
    let separators = [" - ", " – ", " — "];
    for sep in separators {
        // Find the LAST occurrence so titles containing "-" still work.
        if let Some(idx) = after.rfind(sep) {
            let title = after[..idx].trim().to_string();
            let author = after[idx + sep.len()..].trim().to_string();
            return (number, title, author);
        }
    }
    (number, after.to_string(), String::new())
}

/// Strip Obsidian wikilink brackets and trailing whitespace.
fn clean_text(s: &str) -> String {
    let mut out = s.trim().to_string();
    // [[Title]] -> Title
    while let Some(start) = out.find("[[") {
        if let Some(end) = out[start..].find("]]") {
            let inner = &out[start + 2..start + end];
            // Pipe-aliased wikilinks: [[target|display]]
            let display = inner.split('|').next_back().unwrap_or(inner).to_string();
            let replaced = format!("{}{}{}", &out[..start], display, &out[start + end + 2..]);
            out = replaced;
        } else {
            break;
        }
    }
    out.trim().to_string()
}

/// Body cleanup: strip embedded image refs `![[...]]`, normalize trailing whitespace.
fn clean_body(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for line in s.lines() {
        // Drop Obsidian image embeds entirely.
        if line.trim_start().starts_with("![[") {
            continue;
        }
        // Strip block-reference anchors `^abc123` on their own line.
        if line.trim().starts_with('^') && line.trim().chars().skip(1).all(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    // Convert remaining [[Title]] in body to plain text.
    let mut cleaned = clean_text(&out);
    // Collapse 3+ blank lines to 2.
    while cleaned.contains("\n\n\n") {
        cleaned = cleaned.replace("\n\n\n", "\n\n");
    }
    cleaned.trim().to_string()
}

fn extract_url(line: &str) -> Option<String> {
    // `[label](url)` -> url
    if let Some(open) = line.find("](") {
        if let Some(close_rel) = line[open + 2..].find(')') {
            let url = &line[open + 2..open + 2 + close_rel];
            if url.starts_with("http") {
                return Some(url.to_string());
            }
        }
    }
    // bare url
    let trimmed = line.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        // Stop at whitespace
        let end = trimmed.find(|c: char| c.is_whitespace()).unwrap_or(trimmed.len());
        return Some(trimmed[..end].to_string());
    }
    None
}

/// Try to parse the many date shapes in the source file.
/// Returns ISO `YYYY-MM-DD` on success.
fn parse_date(s: &str) -> Option<String> {
    // Normalize dashes to ASCII hyphen.
    let normalized: String = s
        .chars()
        .map(|c| match c {
            '–' | '—' => '-',
            _ => c,
        })
        .collect();
    let trimmed = normalized.trim();

    // "YYYY Book of the Year"
    if let Some(rest) = trimmed.strip_suffix(" Book of the Year") {
        if let Ok(y) = rest.parse::<i32>() {
            if (1900..2100).contains(&y) {
                return Some(format!("{y:04}-12-31"));
            }
        }
    }

    // M/D/YYYY or M-D-YYYY
    if let Some(iso) = parse_numeric_date(trimmed) {
        return Some(iso);
    }

    // "17th Feb 2023", "23d January 2023", "12th July 2024", "10th September 2022", "15th September 2022"
    if let Some(iso) = parse_ordinal_date(trimmed) {
        return Some(iso);
    }

    // "June 2020" (month + year only)
    if let Some(iso) = parse_month_year(trimmed) {
        return Some(iso);
    }

    None
}

fn parse_numeric_date(s: &str) -> Option<String> {
    // Accept "/" or "-" separators.
    let parts: Vec<&str> = s.split(['/', '-']).collect();
    if parts.len() != 3 {
        return None;
    }
    let nums: Option<Vec<i32>> = parts.iter().map(|p| p.trim().parse::<i32>().ok()).collect();
    let nums = nums?;
    // Heuristics:
    //   - if any part > 31: assume that part is the year
    //   - else if last part has 2 digits: 19XX or 20XX based on value
    let (m, d, y) = if nums[0] > 31 {
        (nums[1], nums[2], nums[0]) // YYYY-M-D
    } else if nums[2] > 31 {
        (nums[0], nums[1], nums[2]) // M-D-YYYY
    } else {
        // 2-digit year at end
        let y2 = nums[2];
        let y_full = if y2 < 70 { 2000 + y2 } else { 1900 + y2 };
        (nums[0], nums[1], y_full)
    };
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

fn parse_ordinal_date(s: &str) -> Option<String> {
    // e.g. "17th Feb 2023" -> day, month-name, year
    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.len() != 3 {
        return None;
    }
    // strip suffix from day
    let day_str = tokens[0].trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let day: i32 = day_str.parse().ok()?;
    let month = month_from_name(tokens[1])?;
    let year: i32 = tokens[2].parse().ok()?;
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

fn parse_month_year(s: &str) -> Option<String> {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.len() != 2 {
        return None;
    }
    let month = month_from_name(tokens[0])?;
    let year: i32 = tokens[1].parse().ok()?;
    Some(format!("{year:04}-{month:02}-01"))
}

fn month_from_name(name: &str) -> Option<i32> {
    let lower = name.to_lowercase();
    let lower = lower.trim_end_matches(',');
    Some(match lower {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "sept" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => return None,
    })
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_was_dash = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() { "untitled".to_string() } else { trimmed }
}

fn escape_toml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn render_review(e: &Entry) -> String {
    let mut out = String::new();
    out.push_str("+++\n");
    out.push_str(&format!("number = {}\n", e.number));
    out.push_str(&format!("title = \"{}\"\n", escape_toml(&e.title)));
    out.push_str(&format!("author = \"{}\"\n", escape_toml(&e.author)));
    out.push_str(&format!("date = \"{}\"\n", e.date_iso));
    if !e.date_raw.is_empty() && e.date_raw != e.date_iso {
        out.push_str(&format!("date_raw = \"{}\"\n", escape_toml(&e.date_raw)));
    }
    if !e.link.is_empty() {
        out.push_str(&format!("link = \"{}\"\n", escape_toml(&e.link)));
    }
    out.push_str("+++\n\n");
    out.push_str(&e.body);
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_basic() {
        let (n, t, a) = parse_heading("# 1: Sapiens - Yuval Noah Harari");
        assert_eq!(n, 1);
        assert_eq!(t, "Sapiens");
        assert_eq!(a, "Yuval Noah Harari");
    }

    #[test]
    fn heading_with_wikilink() {
        let (n, t, a) = parse_heading("# 86: [[Elder Race]] - Adrian Tchaikovsky");
        assert_eq!(n, 86);
        assert_eq!(t, "[[Elder Race]]");
        assert_eq!(a, "Adrian Tchaikovsky");
    }

    #[test]
    fn heading_no_author() {
        let (n, t, a) = parse_heading("# 109: Universal Principles of Design");
        assert_eq!(n, 109);
        assert_eq!(t, "Universal Principles of Design");
        assert_eq!(a, "");
    }

    #[test]
    fn heading_title_contains_colon() {
        let (_, t, a) = parse_heading("# 105: The Creative Act: A Way of Being - Rick Rubin");
        assert_eq!(t, "The Creative Act: A Way of Being");
        assert_eq!(a, "Rick Rubin");
    }

    #[test]
    fn date_us_2digit_year() {
        assert_eq!(parse_date("1-09-19").as_deref(), Some("2019-01-09"));
    }

    #[test]
    fn date_endash() {
        assert_eq!(parse_date("2–19–22").as_deref(), Some("2022-02-19"));
    }

    #[test]
    fn date_us_4digit_year() {
        assert_eq!(parse_date("10-8-2020").as_deref(), Some("2020-10-08"));
        assert_eq!(parse_date("7/23/2024").as_deref(), Some("2024-07-23"));
    }

    #[test]
    fn date_ordinal() {
        assert_eq!(parse_date("17th Feb 2023").as_deref(), Some("2023-02-17"));
        assert_eq!(parse_date("23d January 2023").as_deref(), Some("2023-01-23"));
        assert_eq!(parse_date("12th July 2024").as_deref(), Some("2024-07-12"));
    }

    #[test]
    fn date_month_year() {
        assert_eq!(parse_date("June 2020").as_deref(), Some("2020-06-01"));
    }

    #[test]
    fn date_book_of_year() {
        assert_eq!(parse_date("2023 Book of the Year").as_deref(), Some("2023-12-31"));
    }

    #[test]
    fn url_markdown_link() {
        assert_eq!(
            extract_url("[label](https://example.com/x)"),
            Some("https://example.com/x".to_string())
        );
    }

    #[test]
    fn url_bare() {
        assert_eq!(
            extract_url("https://www.goodreads.com/book/show/42046112-recursion"),
            Some("https://www.goodreads.com/book/show/42046112-recursion".to_string())
        );
    }

    #[test]
    fn slug_basic() {
        assert_eq!(slugify("The Creative Act: A Way of Being"), "the-creative-act-a-way-of-being");
        assert_eq!(slugify("13½ Lives"), "13-lives");
    }

    #[test]
    fn clean_wikilink() {
        assert_eq!(clean_text("[[Atomic Habits]]"), "Atomic Habits");
        assert_eq!(clean_text("[[note|Display]]"), "Display");
    }
}
