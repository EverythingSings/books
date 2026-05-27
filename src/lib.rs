//! # books.everythingsings.art
//!
//! Static site generator for personal book reviews. Mirrors the everythingsings.github.io
//! architecture: pure SSR Leptos components rendered at build time, zero client JS.

pub mod components;
pub mod covers;
pub mod parser;

pub mod config {
    pub const SITE_NAME: &str = "Book Reviews";
    pub const SITE_AUTHOR: &str = "EverythingSings";
    pub const SITE_DOMAIN: &str = "books.everythingsings.art";
    pub const SITE_URL: &str = "https://books.everythingsings.art";
    pub const SITE_DESCRIPTION: &str =
        "A personal reading journal — reviews of every book I've finished since 2019.";
}
