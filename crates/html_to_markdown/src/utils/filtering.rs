use std::{collections::HashSet, sync::LazyLock};

static IGNORE_KEYWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "sidebar",
        "author",
        "publication",
        "mobile",
        "share",
        "userinfo",      // author information
        "topics",        // assume tag or topic section
        "comment",       // comments section
        "navigation",    // navigation elements
        "footer",        // footer elements
        "advertisement", // ads or promotional content
        "social",
    ]
    .into_iter()
    .collect()
});

pub fn should_ignore_class(class_value: &str) -> bool {
    if class_value.is_empty() {
        return false;
    }

    let lowercase_class = class_value.to_ascii_lowercase();
    IGNORE_KEYWORDS
        .iter()
        .any(|&keyword| lowercase_class.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// Test case sensitivity
    #[rstest]
    #[case("sidebar", true)]
    #[case("SIDEBAR", true)]
    #[case("Sidebar", true)]
    #[case("SideBar", true)]
    #[case("my-sidebar-widget", true)]
    #[case("author", true)]
    #[case("AUTHOR", true)]
    #[case("publication", true)]
    #[case("PUBLICATION", true)]
    #[case("mobile", true)]
    #[case("MOBILE", true)]
    #[case("note", false)]
    #[case("warning", false)]
    #[case("info", false)]
    #[case("navigation", true)]
    #[case("multiple sidebar classes here", true)]
    #[case("class1 author class2", true)]
    #[case("prefix-mobile-suffix", true)]
    #[case("valid-share-button", true)]
    #[case("widget userinfo panel", true)]
    #[case("content topics section", true)]
    #[case("main comment area", true)]
    #[case("top navigation bar", true)]
    #[case("bottom footer content", true)]
    #[case("ad advertisement space", true)]
    #[case("media social links", true)]
    #[case("note warning info", false)]
    #[case("primary secondary content", false)]
    #[case("main article content", false)]
    #[case("header title text", false)]
    #[case("valid content classes", false)]
    #[case("", false)] // Empty string
    #[case("   ", false)] // Whitespace only
    fn test_should_ignore_class(#[case] class_value: &str, #[case] expected: bool) {
        assert_eq!(should_ignore_class(class_value), expected);
    }
}
