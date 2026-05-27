//! `<head>` rendering. Returns raw HTML (not Leptos `view!`) because Open Graph
//! `property=` attributes are not supported by the macro.

use crate::config::{SITE_AUTHOR, SITE_DESCRIPTION, SITE_NAME, SITE_URL};

const THEME_COLOR: &str = "#0d0d0d";

pub struct PageMeta {
    pub title: String,
    pub description: String,
    pub canonical_url: String,
    pub og_type: String,
    pub og_image: String,
    pub json_ld: String,
}

pub fn generate_head_html(meta: &PageMeta) -> String {
    format!(
        r#"<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>{title}</title>
<meta name="description" content="{description}" />
<meta name="author" content="{author}" />
<link rel="canonical" href="{url}" />
<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32.png" />
<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16.png" />
<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png" />
<link rel="manifest" href="/site.webmanifest" />
<meta name="theme-color" content="{theme}" />
<meta property="og:type" content="{og_type}" />
<meta property="og:title" content="{title}" />
<meta property="og:description" content="{description}" />
<meta property="og:url" content="{url}" />
<meta property="og:image" content="{og_image}" />
<meta property="og:site_name" content="{site_name}" />
<meta name="twitter:card" content="summary" />
<meta name="twitter:title" content="{title}" />
<meta name="twitter:description" content="{description}" />
<meta name="twitter:image" content="{og_image}" />
<link rel="alternate" type="application/rss+xml" title="{site_name} RSS Feed" href="/feed.xml" />
<link rel="stylesheet" href="/main.css" />
<script type="application/ld+json">{json_ld}</script>
</head>"#,
        title = html_escape(&meta.title),
        description = html_escape(&meta.description),
        url = html_escape(&meta.canonical_url),
        og_type = html_escape(&meta.og_type),
        og_image = html_escape(&meta.og_image),
        theme = THEME_COLOR,
        author = SITE_AUTHOR,
        site_name = SITE_NAME,
        json_ld = meta.json_ld,
    )
}

pub fn site_index_json_ld() -> String {
    format!(
        r#"{{
  "@context": "https://schema.org",
  "@type": "Blog",
  "name": "{name}",
  "url": "{url}",
  "description": "{desc}",
  "author": {{
    "@type": "Person",
    "name": "{author}",
    "url": "https://everythingsings.art"
  }}
}}"#,
        name = SITE_NAME,
        url = SITE_URL,
        desc = SITE_DESCRIPTION,
        author = SITE_AUTHOR,
    )
}

pub fn review_json_ld(
    title: &str,
    author: &str,
    date: &str,
    body_excerpt: &str,
    canonical_url: &str,
) -> String {
    format!(
        r#"{{
  "@context": "https://schema.org",
  "@type": "Review",
  "url": "{url}",
  "datePublished": "{date}",
  "author": {{
    "@type": "Person",
    "name": "{reviewer}",
    "url": "https://everythingsings.art"
  }},
  "itemReviewed": {{
    "@type": "Book",
    "name": "{title}",
    "author": {{
      "@type": "Person",
      "name": "{book_author}"
    }}
  }},
  "reviewBody": "{excerpt}"
}}"#,
        url = json_escape(canonical_url),
        date = json_escape(date),
        reviewer = json_escape(SITE_AUTHOR),
        title = json_escape(title),
        book_author = json_escape(author),
        excerpt = json_escape(body_excerpt),
    )
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\n', '\r', '\t'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_contains_required_tags() {
        let meta = PageMeta {
            title: "Sapiens".into(),
            description: "A review".into(),
            canonical_url: "https://books.everythingsings.art/reviews/sapiens/".into(),
            og_type: "article".into(),
            og_image: "https://books.everythingsings.art/covers/sapiens.jpg".into(),
            json_ld: "{}".into(),
        };
        let html = generate_head_html(&meta);
        assert!(html.contains("<title>Sapiens</title>"));
        assert!(html.contains("rel=\"canonical\""));
        assert!(html.contains("og:type"));
        assert!(html.contains("application/rss+xml"));
        assert!(html.contains("application/ld+json"));
    }

    #[test]
    fn html_escape_basic() {
        assert_eq!(html_escape("a & b < c"), "a &amp; b &lt; c");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }
}
