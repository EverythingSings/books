//! Individual review page.

use crate::parser::Review;
use leptos::prelude::*;

#[component]
pub fn ReviewPage(
    review: Review,
    cover_path: Option<String>,
    prev: Option<(String, String)>, // (slug, title)
    next: Option<(String, String)>,
) -> impl IntoView {
    let body_html = review.body_html.clone();
    let title = review.title.clone();
    let author = review.author.clone();
    let date_display = review.date_display.clone();
    let date_iso = review.date.clone();
    let n = review.number;
    let link = review.link.clone();
    let cover = cover_path;
    // Two-column (cover beside text) only when a cover exists; otherwise the
    // text column stands alone at normal reading width.
    let layout_class = if cover.is_some() {
        "review-layout review-layout--with-cover"
    } else {
        "review-layout"
    };

    view! {
        <body itemscope itemtype="https://schema.org/Review">
            <main class="container">
                <nav class="site-nav top-nav">
                    <a href="/">"\u{2190} all reviews"</a>
                </nav>
                <article class="review">
                    <div class=layout_class>
                        {cover.as_ref().map(|c| view! {
                            <figure class="review-cover-wrap">
                                // width/height encode the 2:3 ratio for engines
                                // without aspect-ratio; the CSS aspect-ratio +
                                // object-fit reserves the box before load so
                                // the body never shifts. Cover is above the
                                // fold here, so load it eagerly (it's the LCP).
                                <img
                                    class="review-cover"
                                    src=c.clone()
                                    alt={format!("Cover of {title}")}
                                    width="400"
                                    height="600"
                                    loading="eager"
                                    decoding="async"
                                />
                            </figure>
                        })}
                        <div class="review-main">
                            <header class="review-header">
                                <p class="review-number">{format!("#{n:03}")}</p>
                                <h1 class="review-title" itemprop="itemReviewed" itemscope itemtype="https://schema.org/Book">
                                    <span itemprop="name">{title.clone()}</span>
                                </h1>
                                {(!author.is_empty()).then(|| view! {
                                    <p class="review-author">
                                        "by "
                                        <span itemprop="author" itemscope itemtype="https://schema.org/Person">
                                            <span itemprop="name">{author.clone()}</span>
                                        </span>
                                    </p>
                                })}
                                <p class="review-meta">
                                    <time itemprop="datePublished" datetime=date_iso>{date_display}</time>
                                    {(!link.is_empty()).then(|| view! {
                                        " · "
                                        <a class="external" href=link.clone() rel="noopener external">"\u{2197} source"</a>
                                    })}
                                </p>
                            </header>
                            <div class="review-body" itemprop="reviewBody" inner_html=body_html></div>
                        </div>
                    </div>
                </article>
                <nav class="review-nav">
                    {prev.map(|(slug, t)| view! {
                        <a class="prev" href={format!("/reviews/{slug}/")}>
                            <span class="dir">"\u{2190} previous"</span>
                            <span class="t">{t}</span>
                        </a>
                    })}
                    {next.map(|(slug, t)| view! {
                        <a class="next" href={format!("/reviews/{slug}/")}>
                            <span class="dir">"next \u{2192}"</span>
                            <span class="t">{t}</span>
                        </a>
                    })}
                </nav>
            </main>
        </body>
    }
}
