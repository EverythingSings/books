use books::components::head::{generate_head_html, review_json_ld, PageMeta};
use books::components::{IndexPage, IndexPageProps, ReviewPage, ReviewPageProps};
use books::parser::load_all;
use leptos::prelude::*;
use std::{fs, time::SystemTime};

#[test]
fn year_navigation_and_tags_render_from_review_files() {
    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("books-navigation-{unique}"));
    fs::create_dir(&dir).unwrap();
    for (number, date, tags) in [
        (1, "2019-01-01", ""),
        (
            2,
            "2026-01-01",
            "tags = [\"Science Fiction\", \"Robotics\"]\n",
        ),
        (3, "2026-08-26", ""),
        (4, "2031-02-01", ""),
    ] {
        fs::write(
            dir.join(format!("{number:03}-book-{number}.md")),
            format!("+++\nnumber = {number}\ntitle = \"Book {number}\"\ndate = \"{date}\"\n{tags}+++\n\nA review.\n"),
        ).unwrap();
    }
    let reviews = load_all(&dir).unwrap();
    fs::remove_dir_all(&dir).unwrap();
    assert_eq!(reviews.len(), 4);
    assert!(reviews[0].tags.is_empty());
    assert_eq!(reviews[1].tags, ["Science Fiction", "Robotics"]);

    let index = IndexPage(IndexPageProps {
        reviews: reviews.clone(),
    })
    .to_html();
    for year in [2019, 2026, 2031] {
        assert_eq!(index.matches(&format!("href=\"#year-{year}\"")).count(), 1);
        assert_eq!(index.matches(&format!("id=\"year-{year}\"")).count(), 1);
    }
    assert!(!index.contains("#year-2020"));
    assert!(!index.contains("era-"));
    let first_2026 = index.find("id=\"year-2026\"").unwrap();
    assert!(first_2026 < index.find("href=\"/reviews/book-2/\"").unwrap());
    let page = |i: usize| {
        ReviewPage(ReviewPageProps {
            review: reviews[i].clone(),
            cover_path: None,
            prev: None,
            next: None,
        })
        .to_html()
    };
    assert!(!page(0).contains("review-tags"));
    let tagged = page(1);
    assert!(tagged.contains("aria-label=\"Book tags\""));
    assert!(tagged.contains("itemprop=\"keywords\">Science Fiction</li>"));
    assert!(tagged.contains("itemprop=\"keywords\">Robotics</li>"));
}

#[test]
fn tag_metadata_preserves_labels_and_escapes_markup() {
    let tags = vec![
        "History & Politics".to_string(),
        "Quoted \"tag\" </script>".to_string(),
    ];
    let json_ld = review_json_ld(
        "Book",
        "Author",
        "2026-01-01",
        "Review",
        "https://example.com",
        &tags,
    );
    let parsed: serde_json::Value = serde_json::from_str(&json_ld).unwrap();
    assert_eq!(parsed["keywords"], serde_json::json!(tags));
    assert!(!json_ld.contains("</script>"));
    let head = generate_head_html(&PageMeta {
        title: "Book".into(),
        description: "Review".into(),
        canonical_url: "https://example.com".into(),
        og_type: "article".into(),
        og_image: "https://example.com/cover.jpg".into(),
        json_ld,
        tags,
    });
    assert!(head.contains("content=\"History &amp; Politics\""));
    assert!(head.contains("Quoted &quot;tag&quot; &lt;/script&gt;"));
    assert_eq!(head.matches("property=\"article:tag\"").count(), 2);
}
