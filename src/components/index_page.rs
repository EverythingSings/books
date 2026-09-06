//! Index page: chronological list of all reviews.

use crate::components::head::html_escape;
use crate::parser::{today_display, Review};
use leptos::prelude::*;
use std::collections::BTreeSet;

/// Inline line-icon for a not-yet-written review (a dashed, hollow ring).
const ICON_PENDING: &str = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" \
stroke-width=\"2\" stroke-linecap=\"round\" aria-hidden=\"true\">\
<circle cx=\"12\" cy=\"12\" r=\"9\" stroke-dasharray=\"2.4 3.6\"/></svg>";

/// Inline line-icon for a retroactive review (a clock with a rewind arrow —
/// the Lucide "history" glyph).
const ICON_RETRO: &str = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" \
stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\">\
<path d=\"M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8\"/>\
<path d=\"M3 3v5h5\"/><path d=\"M12 7v5l4 2\"/></svg>";

/// Up/down chevrons for the floating top/bottom navigator.
const ICON_UP: &str = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" \
stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\">\
<path d=\"m18 15-6-6-6 6\"/></svg>";
const ICON_DOWN: &str = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" \
stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\">\
<path d=\"m6 9 6 6 6-6\"/></svg>";

/// Trim a plain-text body to roughly `chars` characters at a word boundary
/// and append an ellipsis if anything was cut.
fn truncate_excerpt(text: &str, chars: usize) -> String {
    if text.chars().count() <= chars {
        return text.to_string();
    }
    let head: String = text.chars().take(chars).collect();
    match head.rsplit_once(' ') {
        Some((before_last_space, _)) => format!("{}…", before_last_space.trim_end()),
        None => format!("{head}…"),
    }
}

#[component]
pub fn IndexPage(reviews: Vec<Review>) -> impl IntoView {
    let total = reviews.len();
    let last_updated = today_display();

    // Each year links to its first review. Derive years from the dates so new
    // years appear automatically, without duplicate anchors or empty sections.
    let mut years = BTreeSet::new();
    let anchored: Vec<Option<String>> = reviews
        .iter()
        .map(|review| {
            let year = review.date.get(..4)?;
            if year.bytes().all(|c| c.is_ascii_digit()) && years.insert(year.to_string()) {
                Some(format!("year-{year}"))
            } else {
                None
            }
        })
        .collect();

    let entries: Vec<_> = reviews
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let href = format!("/reviews/{}/", r.slug);
            let title = r.title.clone();
            let author = r.author.clone();
            let date = r.date_display.clone();
            let n = r.number;
            let preview = truncate_excerpt(&r.body_text, 220);
            let anchor = anchored[i].clone();
            // Pending takes precedence: a stub has no review, so it can't be
            // "retroactive" (the parser already enforces this).
            let status = if r.pending {
                Some(
                    view! {
                        <span class="entry-status pending" role="img"
                            aria-label="Review pending" title="Review pending"
                            inner_html=ICON_PENDING></span>
                    }
                    .into_any(),
                )
            } else if r.retroactive {
                let label = if r.reviewed_display.is_empty() {
                    "Retroactive review".to_string()
                } else {
                    format!(
                        "Finished {}, reviewed {}",
                        r.date_display, r.reviewed_display
                    )
                };
                Some(
                    view! {
                        <span class="entry-status retroactive" role="img"
                            aria-label=label.clone() title=label
                            inner_html=ICON_RETRO></span>
                    }
                    .into_any(),
                )
            } else {
                None
            };
            view! {
                <li class="entry" id=anchor>
                    <a class="entry-link" href=href>
                        <span class="entry-num">{format!("#{n:03}")}</span>
                        <span class="entry-title">{title}{status}</span>
                        {(!author.is_empty()).then(|| view! {
                            <span class="entry-author">{author}</span>
                        })}
                        <span class="entry-date">{date}</span>
                        <span class="entry-preview">{preview}</span>
                    </a>
                </li>
            }
        })
        .collect();

    let year_links: Vec<_> = years
        .into_iter()
        .map(|year| {
            let target = format!("#year-{year}");
            view! { <li><a href=target>{year}</a></li> }
        })
        .collect();

    view! {
        <body itemscope itemtype="https://schema.org/Blog">
            <div class="scroll-progress" aria-hidden="true"></div>
            <nav class="floating-nav" aria-label="Jump to top or bottom">
                <a class="floating-nav-btn" href="#top" aria-label="Return to top">
                    <span class="floating-nav-icon" aria-hidden="true" inner_html=ICON_UP></span>
                    <span class="floating-nav-label">"Top"</span>
                </a>
                <a class="floating-nav-btn" href="#bottom" aria-label="Jump to bottom">
                    <span class="floating-nav-icon" aria-hidden="true" inner_html=ICON_DOWN></span>
                    <span class="floating-nav-label">"Bottom"</span>
                </a>
            </nav>
            <header class="site-hero" role="banner">
                <div class="site-hero-bg" aria-hidden="true"></div>
                <div class="site-hero-content">
                    <div class="hero-card">
                        <h1 class="site-title">"Book Reviews"</h1>
                        <p class="hero-about">
                            "Books are a form of time travel. Open one and you\u{2019}re inside a mind from two hundred years ago, or a thousand. Writing about what I read is another layer of that. These reviews are what I send forward. Layered time travel."
                        </p>
                        <nav class="year-nav" aria-labelledby="year-nav-title">
                            <h2 id="year-nav-title">"Contents by year"</h2>
                            <ol class="year-links">{year_links}</ol>
                        </nav>
                        <p class="hero-updated">
                            <span class="hero-updated-label">"Last updated"</span>
                            <span class="hero-updated-date">{last_updated}</span>
                            <span class="hero-updated-version">{format!("v{}", env!("CARGO_PKG_VERSION"))}</span>
                        </p>
                    </div>
                </div>
            </header>
            <main class="container">
                <header class="site-header">
                    <nav class="site-nav">
                        <a href="https://everythingsings.art" rel="me">"\u{2190} everythingsings.art"</a>
                        <a href="/feed.xml">"RSS"</a>
                        <a href="#bottom">"bottom \u{2193}"</a>
                    </nav>
                </header>
                <details class="status-legend">
                    <summary class="status-legend-summary">"About these reviews, how they\u{2019}re written & their status marks"</summary>
                    <div class="status-legend-body">
                        <p>
                            "The aim is for every book here to carry a written review. That isn\u{2019}t always immediate — sometimes a book is logged first and the review follows, or is backfilled long after the reading. Two small marks show where an entry stands:"
                        </p>
                        <ul class="status-legend-list">
                            <li>
                                <span class="entry-status pending" aria-hidden="true" inner_html=ICON_PENDING></span>
                                <span><strong>"Review pending"</strong>" — logged, with the review still to come."</span>
                            </li>
                            <li>
                                <span class="entry-status retroactive" aria-hidden="true" inner_html=ICON_RETRO></span>
                                <span><strong>"Retroactive review"</strong>" — written some time after the book was read."</span>
                            </li>
                        </ul>
                        <p class="status-legend-note">
                            "How these come together varies. Some are written by hand; others are edited with AI from a voice-to-text transcription of me talking through the book."
                        </p>
                    </div>
                </details>
                <ol class="entries" reversed=false>
                    {entries}
                </ol>
                <footer class="site-footer" id="bottom">
                    <p>
                        {format!("{total} reviews · oldest first · ")}
                        <a href="#top">"top \u{2191}"</a>
                    </p>
                    <p class="footer-formats">
                        "Machine-readable: "
                        <a href="/llms-full.txt">"llms-full.txt"</a>
                        " (every review, full text) · "
                        <a href="/llms.txt">"llms.txt"</a>
                        " · "
                        <a href="/feed.xml">"RSS"</a>
                        " · "
                        <a href="/sitemap.xml">"sitemap"</a>
                    </p>
                </footer>
            </main>
        </body>
    }
}

/// Renders the index page as plain text (for llms.txt and similar).
pub fn render_index_text(reviews: &[Review]) -> String {
    // No top-level header here: the only caller (llms.txt) prints its own.
    let mut out = String::new();
    for r in reviews {
        if r.author.is_empty() {
            out.push_str(&format!("- #{:03} {} ({})\n", r.number, r.title, r.date));
        } else {
            out.push_str(&format!(
                "- #{:03} {} — {} ({})\n",
                r.number, r.title, r.author, r.date
            ));
        }
    }
    out
}

/// Index page also gets a microdata-ready summary for screen readers / crawlers.
pub fn render_index_microdata(reviews: &[Review]) -> String {
    let items: Vec<String> = reviews
        .iter()
        .map(|r| {
            format!(
                "  <li itemprop=\"blogPost\" itemscope itemtype=\"https://schema.org/BlogPosting\">\
                <a itemprop=\"url\" href=\"/reviews/{slug}/\">\
                <span itemprop=\"headline\">{title}</span></a>\
                <meta itemprop=\"datePublished\" content=\"{date}\"/></li>",
                slug = html_escape(&r.slug),
                title = html_escape(&r.title),
                date = html_escape(&r.date),
            )
        })
        .collect();
    items.join("\n")
}
