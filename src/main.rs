//! Static site generator entrypoint.
//!
//! Usage:
//!   cargo run -- --generate-static
//!
//! Set `BOOKS_FETCH_COVERS=1` to fetch missing covers from OpenLibrary
//! during the build (uncached covers only — already-fetched files are reused).

use books::components::head::{
    generate_head_html, review_json_ld, site_index_json_ld, PageMeta,
};
use books::components::index_page::{render_index_microdata, render_index_text};
use books::components::{IndexPage, IndexPageProps, ReviewPage, ReviewPageProps};
use books::config::{SITE_AUTHOR, SITE_DESCRIPTION, SITE_NAME, SITE_URL};
use books::covers::{covers_dir_under, CoverCache};
use books::parser::{load_all, Review};

/// (slug, title) pair for a previous/next review link.
type Sibling = Option<(String, String)>;
use leptos::prelude::*;
use std::env;
use std::fs;
use std::path::Path;

const OUTPUT: &str = "target/site";
const REVIEWS_DIR: &str = "reviews";
const PUBLIC_DIR: &str = "public";
const STYLE_FILE: &str = "style/main.css";

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("--help");
    match cmd {
        "--generate-static" => {
            if let Err(e) = generate() {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        _ => {
            println!("usage: books --generate-static");
            println!("env:   BOOKS_FETCH_COVERS=1 to fetch missing book covers from OpenLibrary");
        }
    }
}

fn generate() -> std::io::Result<()> {
    let out = Path::new(OUTPUT);
    let public = Path::new(PUBLIC_DIR);
    fs::create_dir_all(out)?;

    // Load reviews
    let reviews_path = Path::new(REVIEWS_DIR);
    let reviews = load_all(reviews_path)?;
    println!("loaded {} reviews from {}", reviews.len(), reviews_path.display());

    // Cover cache (writes into public/covers/ so files are part of the source tree).
    let covers_dir = covers_dir_under(public);
    let covers = CoverCache::new(covers_dir);

    // Copy public assets
    if public.exists() {
        copy_dir(public, out)?;
        println!("copied {} -> {}", public.display(), out.display());
    }

    // Copy main.css
    let css_src = Path::new(STYLE_FILE);
    if css_src.exists() {
        fs::copy(css_src, out.join("main.css"))?;
        println!("copied {} -> {}/main.css", css_src.display(), out.display());
    }

    // Build "previous"/"next" pairs for navigation.
    let mut prev_next: Vec<(Sibling, Sibling)> = Vec::with_capacity(reviews.len());
    for i in 0..reviews.len() {
        let prev = if i > 0 {
            Some((reviews[i - 1].slug.clone(), reviews[i - 1].title.clone()))
        } else {
            None
        };
        let next = if i + 1 < reviews.len() {
            Some((reviews[i + 1].slug.clone(), reviews[i + 1].title.clone()))
        } else {
            None
        };
        prev_next.push((prev, next));
    }

    // Generate each review page
    for (i, review) in reviews.iter().enumerate() {
        let cover_path = covers.cover_path(&review.slug, &review.title, &review.author);
        let html = render_review_page(review, cover_path.as_deref(), &prev_next[i]);
        let dir = out.join("reviews").join(&review.slug);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), html)?;
    }
    println!("generated {} review pages", reviews.len());

    // Generate index
    let index_html = render_index_page(&reviews);
    fs::write(out.join("index.html"), index_html)?;
    println!("generated index.html");

    // sitemap.xml
    fs::write(out.join("sitemap.xml"), generate_sitemap(&reviews))?;

    // feed.xml
    fs::write(out.join("feed.xml"), generate_feed(&reviews))?;

    // llms.txt
    fs::write(out.join("llms.txt"), generate_llms_txt(&reviews))?;

    println!("\ndone -> {}", out.display());
    Ok(())
}

fn render_index_page(reviews: &[Review]) -> String {
    let head = generate_head_html(&PageMeta {
        title: format!("{SITE_NAME} | {SITE_AUTHOR}"),
        description: SITE_DESCRIPTION.to_string(),
        canonical_url: format!("{SITE_URL}/"),
        og_type: "website".to_string(),
        og_image: format!("{SITE_URL}/og-cover.png"),
        json_ld: site_index_json_ld(),
    });
    let body = IndexPage(IndexPageProps {
        reviews: reviews.to_vec(),
    })
    .to_html();
    let body = strip_leptos_markers(&body);
    let microdata = render_index_microdata(reviews);
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n{head}\n{body}\n<!-- machine-readable index:\n<ol>\n{microdata}\n</ol>\n-->\n</html>\n"
    )
}

/// Leptos emits `<!>` placeholder tags for empty `Option` branches during SSR.
/// We aren't hydrating, so strip them — they're invalid HTML and create visual gaps.
fn strip_leptos_markers(html: &str) -> String {
    html.replace("<!>", "")
}

fn render_review_page(
    review: &Review,
    cover_path: Option<&str>,
    pn: &(Sibling, Sibling),
) -> String {
    let canonical = format!("{SITE_URL}/reviews/{}/", review.slug);
    let og_image = cover_path
        .map(|p| format!("{SITE_URL}{p}"))
        .unwrap_or_else(|| format!("{SITE_URL}/og-cover.png"));
    let excerpt: String = review.body_text.chars().take(280).collect();
    let description = if excerpt.is_empty() {
        format!("Review of {} by {}", review.title, review.author)
    } else {
        excerpt.clone()
    };
    let json_ld = review_json_ld(
        &review.title,
        &review.author,
        &review.date,
        &excerpt,
        &canonical,
    );
    let head = generate_head_html(&PageMeta {
        title: format!("{} — {}", review.title, SITE_NAME),
        description,
        canonical_url: canonical,
        og_type: "article".to_string(),
        og_image,
        json_ld,
    });
    let body = ReviewPage(ReviewPageProps {
        review: review.clone(),
        cover_path: cover_path.map(String::from),
        prev: pn.0.clone(),
        next: pn.1.clone(),
    })
    .to_html();
    let body = strip_leptos_markers(&body);
    format!("<!DOCTYPE html>\n<html lang=\"en\">\n{head}\n{body}\n</html>\n")
}

fn generate_sitemap(reviews: &[Review]) -> String {
    let mut urls = String::new();
    urls.push_str(&format!(
        "  <url><loc>{SITE_URL}/</loc><changefreq>weekly</changefreq><priority>1.0</priority></url>\n"
    ));
    for r in reviews {
        urls.push_str(&format!(
            "  <url><loc>{SITE_URL}/reviews/{}/</loc><lastmod>{}</lastmod><priority>0.7</priority></url>\n",
            r.slug, r.date
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{urls}</urlset>\n"
    )
}

fn generate_feed(reviews: &[Review]) -> String {
    // Newest first for the feed (per Atom/RSS convention) — list page is oldest-first by request.
    let mut sorted = reviews.to_vec();
    sorted.sort_by(|a, b| b.date.cmp(&a.date).then(b.number.cmp(&a.number)));
    let mut items = String::new();
    for r in sorted.iter().take(30) {
        let url = format!("{SITE_URL}/reviews/{}/", r.slug);
        let title = xml_escape(&format!(
            "#{:03} {} — {}",
            r.number,
            r.title,
            if r.author.is_empty() { "—" } else { &r.author }
        ));
        let desc = xml_escape(&r.body_text.chars().take(500).collect::<String>());
        let pub_date = rfc822_date(&r.date);
        items.push_str(&format!(
            "  <item>\n    <title>{title}</title>\n    <link>{url}</link>\n    <guid>{url}</guid>\n    <pubDate>{pub_date}</pubDate>\n    <description>{desc}</description>\n  </item>\n"
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\"><channel>\n  <title>{SITE_NAME}</title>\n  <link>{SITE_URL}</link>\n  <description>{SITE_DESCRIPTION}</description>\n  <language>en</language>\n{items}</channel></rss>\n"
    )
}

fn generate_llms_txt(reviews: &[Review]) -> String {
    let mut out = format!(
        "# {SITE_NAME}\n\n> {SITE_DESCRIPTION}\n\n- URL: {SITE_URL}\n- Author: {SITE_AUTHOR} (https://everythingsings.art)\n- Type: personal reading journal\n- Format: static HTML, no JavaScript required\n- Feed: {SITE_URL}/feed.xml\n\n## All Reviews (oldest first)\n\n"
    );
    out.push_str(&render_index_text(reviews));
    out
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn rfc822_date(iso: &str) -> String {
    // Minimal: convert YYYY-MM-DD to RFC-822 with fixed 00:00:00 GMT.
    let parts: Vec<&str> = iso.split('-').collect();
    if parts.len() != 3 {
        return iso.to_string();
    }
    let year = parts[0];
    let month: usize = parts[1].parse().unwrap_or(1);
    let day: u32 = parts[2].parse().unwrap_or(1);
    let m_name = match month {
        1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
        5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
        9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
        _ => "Jan",
    };
    format!("{day:02} {m_name} {year} 00:00:00 GMT")
}
