# CLAUDE.md

This file provides guidance to Claude Code when working in the `books/` project.

## Canonical location (WSL)

This repo lives in the WSL filesystem and is the **only** working copy:

```
/home/trist/engineering/books   # remote git@github.com:EverythingSings/books.git, branch main
```

A stale duplicate used to exist on the Windows mount
(`/mnt/c/Users/Trist/engineering/books`); it has been deleted. Always work here
under WSL. Deploy = `cargo test && cargo build --release && git push origin main`
(GitHub Actions publishes to GitHub Pages on push to `main`; there is no `gh`
CLI on this machine).

## Project Overview

`books.everythingsings.art` — a personal book-reviews site. Pure SSR with Leptos
(zero client JS), deployed as static HTML to GitHub Pages. Mirrors the
architecture of `everythingsings.github.io`.

## Build Commands

```bash
cargo build --release
./target/release/books --generate-static     # writes target/site/

# local preview
python3 -m http.server 8765 --directory target/site

# unit tests
cargo test

# format / lint
cargo fmt
cargo clippy --all-targets -- -D warnings
```

## Architecture

### Pure SSR (no client JS, no WASM)

- `#[component]` functions render server-side only via `to_html()`
- `crate-type = ["rlib"]` — no `cdylib`, no WASM target
- Components may read from `reviews/` directly because they only run at build time
- After `.to_html()`, the output is post-processed with `strip_leptos_markers`
  to drop the `<!>` placeholders Leptos emits for empty `Option` branches in SSR

### Data flow

```
reviews/NNN-slug.md  (TOML frontmatter + markdown body)
        │
        ▼
parser::load_all       → Vec<Review>           (sorted by review.number)
        │
        ▼
covers::CoverCache     → Option<cover_url>    (cached in public/covers/<slug>.jpg)
        │
        ▼
components::IndexPage  → target/site/index.html
components::ReviewPage → target/site/reviews/<slug>/index.html
main::generate_*       → sitemap.xml, feed.xml, llms.txt
```

### Review file format

Every review lives at `reviews/NNN-slug.md` and looks like:

```toml
+++
number = 1
title = "Sapiens"
author = "Yuval Noah Harari"
date = "2019-01-09"
date_raw = "1-09-19"               # optional — preserved from the original source
link = "https://www.ynharari.com/book/sapiens/"   # optional
+++

The review body, in markdown.
```

`date_raw` and `link` are optional. `author` may be empty (e.g. autobiographies).
Filename `NNN-slug.md` controls the URL slug; `NNN` is zero-padded for natural sort.

### Adding a new review (the daily workflow)

```bash
# pick the next number
N=$(ls reviews/ | tail -n1 | cut -c1-3)
NEXT=$(printf "%03d" $((10#$N + 1)))

# copy a template and edit
cp reviews/100-atomic-habits.md "reviews/${NEXT}-new-book.md"
$EDITOR "reviews/${NEXT}-new-book.md"

# fetch/cache the cover and preview
BOOKS_FETCH_COVERS=1 cargo run --release -- --generate-static
python3 -m http.server 8765 --directory target/site

# ship
git add reviews/ public/covers/ && git commit -m "add review ${NEXT}: <title>" && git push
```

GitHub Actions builds and deploys on push to `main`.

### Importing the original Obsidian master file

A one-shot `import` binary parses the original `Book Reviews.md` and emits
per-review files. Re-runnable (it overwrites the `reviews/` directory contents,
including hand-edits — be careful):

```bash
cargo run --bin import -- "/path/to/Book Reviews.md" reviews/
```

The importer handles the messy date formats in the source file (`M-D-YY`,
`M-D-YYYY`, `Month YYYY`, `Nth Month YYYY`, `YYYY Book of the Year`, en/em-dash
variants) and strips Obsidian wikilink and image-embed syntax.

### Book covers

Every review should display a real book cover whenever one is discoverable. Cover
lookup is best-effort and must never block publishing a review.

Covers are fetched from Open Library at build time, opt-in locally:

```bash
BOOKS_FETCH_COVERS=1 cargo run --release -- --generate-static
```

Strategy:

- `public/covers/<slug>.jpg` exists → use it (no network call)
- Else: search multiple Open Library results by title and author, then retry by
  title, fetch `covers.openlibrary.org/b/id/<id>-L.jpg`, and write the JPG
- If no cover is found or the service is unavailable, publish without a cover
  and retry on a future cover-enabled build; do not create permanent miss markers

Commit fetched files in `public/covers/` so subsequent builds don't re-fetch.
The generator resolves covers before copying `public/`, ensuring a newly fetched
image is included in the same static artifact.

### Semantic markup layers (matches everythingsings.github.io)

1. **JSON-LD** in `<head>` — Schema.org `Blog` on the index, `Review` containing
   `Book` + `Person` on each entry
2. **Schema.org microdata** via `itemscope`/`itemprop` on the body — the index
   page also emits a machine-readable `<!--`-wrapped microdata mirror that AI
   crawlers can find via View Source
3. **RSS feed** at `/feed.xml`, newest-first, 30 most recent reviews
4. **`/llms.txt`** — markdown sitemap of all reviews for LLM consumption
5. **`/sitemap.xml`** + permissive `robots.txt` (ClaudeBot, GPTBot, etc. explicitly allowed)

### Required static files (mirrors everythingsings.github.io)

| File | Purpose |
|------|---------|
| `/llms.txt` | AI-optimized markdown sitemap |
| `/robots.txt` | Explicitly allow GPTBot, ClaudeBot, PerplexityBot |
| `/feed.xml` | RSS feed |
| `/sitemap.xml` | XML sitemap |
| `CNAME` | `books.everythingsings.art` |

## Deployment

`.github/workflows/deploy.yml` runs `cargo test`, `cargo build --release`, then
generates the site with `BOOKS_FETCH_COVERS=1`. Fetched covers are cached between
workflow runs and included in the Pages artifact. A failed lookup does not fail
the deployment. DNS `CNAME` for `books.everythingsings.art` must point at
`<github-username>.github.io` (or to the Pages CNAME).
