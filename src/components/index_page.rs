//! Index page: chronological list of all reviews.

use crate::components::head::html_escape;
use crate::parser::Review;
use leptos::prelude::*;

#[component]
pub fn IndexPage(reviews: Vec<Review>) -> impl IntoView {
    let total = reviews.len();
    let entries: Vec<_> = reviews
        .iter()
        .map(|r| {
            let href = format!("/reviews/{}/", r.slug);
            let title = r.title.clone();
            let author = r.author.clone();
            let date = r.date_display.clone();
            let n = r.number;
            view! {
                <li class="entry">
                    <a class="entry-link" href=href>
                        <span class="entry-num">{format!("#{n:03}")}</span>
                        <span class="entry-title">{title}</span>
                        {(!author.is_empty()).then(|| view! {
                            <span class="entry-author">{author}</span>
                        })}
                        <span class="entry-date">{date}</span>
                    </a>
                </li>
            }
        })
        .collect();

    view! {
        <body itemscope itemtype="https://schema.org/Blog">
            <main class="container">
                <header class="site-header">
                    <h1 class="site-title">"Book Reviews"</h1>
                    <p class="site-subtitle">
                        "A personal reading journal. Books are time capsules — these are mine."
                    </p>
                    <nav class="site-nav">
                        <a href="https://everythingsings.art" rel="me">"\u{2190} everythingsings.art"</a>
                        <a href="/feed.xml">"RSS"</a>
                    </nav>
                    <aside class="kindle-callout">
                        <p class="kc-eyebrow">"Also available as a book"</p>
                        <p class="kc-body">
                            "The first 101 reviews are collected in a Kindle edition."
                        </p>
                        <a class="kc-cta" href="https://www.amazon.com/dp/B0BTXZPBYF" rel="noopener external">
                            "Read on Amazon \u{2192}"
                        </a>
                    </aside>
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
