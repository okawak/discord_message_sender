use html_to_markdown::convert;

#[test]
fn test_qiita_article_extraction() {
    let mut html = include_str!("qiita_test_data1.html");
    if html.is_empty() {
        html = "<p>info</p>";
    }
    let url = "https://qiita.com/username/items/1234567890abcdef";
    let keys = ["title", "source"];

    let result = convert(url, html, &keys).expect("failed to convert HTML");

    let mut it = result.splitn(3, "---");
    it.next().unwrap(); // remove front-matter
    it.next().unwrap();
    let body = it.next().unwrap();

    assert!(body.trim_start().starts_with("info"))
}
