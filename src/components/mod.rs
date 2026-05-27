pub mod head;
pub mod index_page;
pub mod review_page;

pub use head::{generate_head_html, PageMeta};
pub use index_page::{IndexPage, IndexPageProps};
pub use review_page::{ReviewPage, ReviewPageProps};
