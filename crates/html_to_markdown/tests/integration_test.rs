use html_to_markdown::convert;
use std::{fs, path::PathBuf};

const TRACKED_ARTICLE_HTML: &str = include_str!("fixtures/article.html");
const TRACKED_ARTICLE_MARKDOWN: &str = include_str!("fixtures/article.md");

fn read_from_crate(rel: &str) -> std::io::Result<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(path)
}

fn load_optional_file(filename: &str) -> String {
    read_from_crate(&format!("tests/{filename}"))
        .unwrap_or_else(|error| panic!("optional fixture tests/{filename} is unavailable: {error}"))
}

#[test]
fn converts_tracked_article_fixture() {
    let url = "https://example.com/articles/current";
    let result = convert(url, TRACKED_ARTICLE_HTML, &["title", "source"])
        .expect("failed to convert tracked article fixture");

    assert_eq!(
        result.trim_end_matches('\n'),
        TRACKED_ARTICLE_MARKDOWN.trim_end_matches('\n')
    );
}

#[test]
#[ignore = "requires the local Qiita full-page fixture"]
fn converts_optional_qiita_article_fixture() {
    let html = load_optional_file("qiita_test_data1.html");
    let url = "https://qiita.com/username/items/1234567890abcdef";
    let keys = ["title", "source"];

    let result = convert(url, &html, &keys).expect("failed to convert HTML");

    let mut it = result.splitn(3, "---");
    it.next().unwrap(); // remove front-matter
    it.next().unwrap();
    let body = it.next().unwrap();

    assert!(body.trim_start().starts_with("info"))
}

#[test]
#[ignore = "requires the local Zenn full-page fixtures"]
fn converts_optional_zenn_article_fixture() {
    let html = load_optional_file("zenn_test_data1.html");
    let url = "https://zenn.com/username/items/1234567890abcdef";
    let keys = ["title", "source"];

    let result = convert(url, &html, &keys).expect("failed to convert HTML");

    let mut it = result.splitn(3, "---");
    it.next().unwrap(); // remove front-matter
    it.next().unwrap();
    let body = it.next().unwrap();

    let markdown = load_optional_file("zenn_test_result1.md");

    assert_eq!(body, markdown)
}
