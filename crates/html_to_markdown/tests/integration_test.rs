use html_to_markdown::convert;
use std::{fs, path::PathBuf};

fn read_from_crate(rel: &str) -> std::io::Result<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(path)
}

fn load_file(filename: &str) -> String {
    read_from_crate(&format!("tests/{filename}")).unwrap_or_default()
}

#[test]
fn test_qiita_article_extraction() {
    let html = load_file("qiita_test_data1.html");
    if html.is_empty() {
        return;
    }
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
fn test_zenn_article1_extraction() {
    let html = load_file("zenn_test_data1.html");
    if html.is_empty() {
        return;
    }
    let url = "https://zenn.com/username/items/1234567890abcdef";
    let keys = ["title", "source"];

    let result = convert(url, &html, &keys).expect("failed to convert HTML");

    let mut it = result.splitn(3, "---");
    it.next().unwrap(); // remove front-matter
    it.next().unwrap();
    let body = it.next().unwrap();

    let markdown = load_file("zenn_test_result1.md");

    assert_eq!(body, markdown)
}
