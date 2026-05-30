//! Index page: chronological list of all reviews.

use crate::components::head::html_escape;
use crate::parser::{today_display, Review};
use leptos::prelude::*;

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

/// Reading eras, each anchored to the first review whose year falls within it.
/// `start_year` partitions the timeline: a review belongs to the latest era
/// whose `start_year` is <= the review's year.
struct Era {
    start_year: u32,
    years: &'static str,
    desc: &'static str,
}

const ERAS: &[Era] = &[
    Era { start_year: 2019, years: "2019–2020", desc: "philosophy & self-discovery" },
    Era { start_year: 2020, years: "2020–2022", desc: "deep sci-fi immersion" },
    Era { start_year: 2023, years: "2023–2024", desc: "AI, design & creativity" },
    Era { start_year: 2024, years: "2024–2025", desc: "the inward turn" },
];

/// Year (first four chars of the ISO date) → index into `ERAS`.
fn era_index(date: &str) -> usize {
    let year: u32 = date.get(..4).and_then(|y| y.parse().ok()).unwrap_or(0);
    ERAS.iter()
        .rposition(|e| e.start_year <= year)
        .unwrap_or(0)
}

#[component]
pub fn IndexPage(reviews: Vec<Review>) -> impl IntoView {
    let total = reviews.len();
    let last_updated = today_display();

    // The first review in each era gets an `id="era-N"` anchor so the header
    // links can jump straight to that section. `anchored[i]` is the era index
    // to stamp on entry `i`, if it's the first entry of its era.
    let mut anchored: Vec<Option<usize>> = vec![None; reviews.len()];
    let mut seen = [false; 8];
    for (i, r) in reviews.iter().enumerate() {
        let e = era_index(&r.date);
        if !seen[e] {
            seen[e] = true;
            anchored[i] = Some(e);
        }
    }
    let era_present = seen;

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
            let anchor = anchored[i].map(|e| format!("era-{e}"));
            view! {
                <li class="entry" id=anchor>
                    <a class="entry-link" href=href>
                        <span class="entry-num">{format!("#{n:03}")}</span>
                        <span class="entry-title">{title}</span>
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

    let eras: Vec<_> = ERAS
        .iter()
        .enumerate()
        .map(|(e, era)| {
            let years = era.years;
            let desc = era.desc;
            // Only link eras that actually contain a review.
            if era_present[e] {
                let target = format!("#era-{e}");
                view! {
                    <dt><a class="era-link" href=target.clone()>{years}</a></dt>
                    <dd><a class="era-link" href=target>{desc}</a></dd>
                }
                .into_any()
            } else {
                view! { <dt>{years}</dt><dd>{desc}</dd> }.into_any()
            }
        })
        .collect();

    view! {
        <body itemscope itemtype="https://schema.org/Blog">
            <div class="scroll-progress" aria-hidden="true"></div>
            <header class="site-hero" role="banner">
                <div class="site-hero-bg" aria-hidden="true"></div>
                <div class="site-hero-content">
                    <div class="hero-card">
                        <h1 class="site-title">"Book Reviews"</h1>
                        <p class="hero-about">
                            "Books are a form of time travel. Open one and you\u{2019}re inside a mind from two hundred years ago, or a thousand. Writing about what I read is another layer of that. These reviews are what I send forward. Layered time travel."
                        </p>
                        <dl class="era-list">
                            {eras}
                        </dl>
                        <p class="hero-updated">
                            <span class="hero-updated-label">"Last updated"</span>
                            <span class="hero-updated-date">{last_updated}</span>
                        </p>
                    </div>
                </div>
            </header>
            <main class="container">
                <header class="site-header">
                    <nav class="site-nav">
                        <a href="https://everythingsings.art" rel="me">"\u{2190} everythingsings.art"</a>
                        <a href="/feed.xml">"RSS"</a>
                    </nav>
                </header>
                <ol class="entries" reversed=false>
                    {entries}
                </ol>
                <footer class="site-footer">
                    <p>
                        {format!("{total} reviews · oldest first · ")}
                        <a href="#top">"top \u{2191}"</a>
                    </p>
                </footer>
            </main>
        </body>
    }
}

/// Renders the index page as plain text (for llms.txt and similar).
pub fn render_index_text(reviews: &[Review]) -> String {
    let mut out = String::new();
    out.push_str("# Book Reviews\n\n");
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
