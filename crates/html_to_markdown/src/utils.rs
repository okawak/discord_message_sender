use std::borrow::Cow;

/// Normalization utilities for HTML text and content
pub fn normalize_html_text(text: &str, preserve_edge_spaces: bool) -> Option<Cow<'_, str>> {
    if text.trim().is_empty() {
        return None;
    }

    let needs_char_filtering = text.chars().any(|c| {
        matches!(
            c,
            '\u{00A0}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
        ) || (c.is_control() && !matches!(c, '\t' | '\n' | '\r'))
    });

    let needs_whitespace_norm = has_extra_whitespace(text);

    if !needs_char_filtering && !needs_whitespace_norm && !preserve_edge_spaces {
        return Some(Cow::Borrowed(text));
    }

    let processed = if needs_char_filtering {
        let filtered: String = text
            .chars()
            .filter(|c| {
                !matches!(
                    *c,
                    '\u{00A0}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
                ) && (!c.is_control() || matches!(*c, '\t' | '\n' | '\r'))
            })
            .collect();

        if needs_whitespace_norm {
            Cow::Owned(filtered.split_whitespace().collect::<Vec<_>>().join(" "))
        } else {
            Cow::Owned(filtered)
        }
    } else if needs_whitespace_norm {
        Cow::Owned(text.split_whitespace().collect::<Vec<_>>().join(" "))
    } else {
        Cow::Borrowed(text)
    };

    if processed.trim().is_empty() {
        return None;
    }

    if preserve_edge_spaces {
        let needs_edge_adjustment =
            text.starts_with(char::is_whitespace) || text.ends_with(char::is_whitespace);

        if needs_edge_adjustment {
            let leading = text.starts_with(char::is_whitespace);
            let trailing = text.ends_with(char::is_whitespace);

            let adjusted = match (leading, trailing) {
                (true, true) => format!(" {} ", processed.trim()),
                (true, false) => format!(" {}", processed.trim()),
                (false, true) => format!("{} ", processed.trim()),
                (false, false) => processed.into_owned(),
            };
            Some(Cow::Owned(adjusted))
        } else {
            Some(processed)
        }
    } else {
        Some(processed)
    }
}

/// Normalizes heading content by removing extra whitespace
pub fn normalize_heading_content(content: &str) -> Cow<'_, str> {
    let needs_br_replacement = content.contains("<br>");
    let needs_newline_replacement = content.contains('\n') || content.contains('\r');

    if needs_br_replacement || needs_newline_replacement {
        Cow::Owned(
            content
                .replace("<br>", " ")
                .replace(['\r', '\n'], " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
        )
    } else if has_extra_whitespace(content) {
        Cow::Owned(content.split_whitespace().collect::<Vec<_>>().join(" "))
    } else {
        Cow::Borrowed(content)
    }
}

fn has_extra_whitespace(text: &str) -> bool {
    let chars = text.chars();
    let mut prev_was_space = false;

    for c in chars {
        if c.is_whitespace() {
            if prev_was_space || c != ' ' {
                return true;
            }
            prev_was_space = true;
        } else {
            prev_was_space = false;
        }
    }
    false
}

#[inline]
pub fn cow_to_string(cow: Cow<'_, str>) -> String {
    cow.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case("", None)]
    #[case("   ", None)]
    #[case("Hello World", Some("Hello World"))]
    #[case("  Hello   World  ", Some("Hello World"))]
    #[case("\u{00A0}\u{200B}Hello\u{FEFF}World\u{2060}", Some("HelloWorld"))]
    #[case("Multi\nLine\tText", Some("Multi Line Text"))]
    fn test_normalize_html_text(#[case] input: &str, #[case] expected: Option<&str>) {
        let result = normalize_html_text(input, false);
        match (&result, expected) {
            (Some(cow), Some(exp)) => assert_eq!(cow.as_ref(), exp),
            (None, None) => {} // Both are None
            _ => panic!("Unexpected result: {result:?}, expected: {expected:?}"),
        }
    }

    /// formatting tests for heading content normalization
    #[rstest]
    #[case("Multi\nLine\nTitle", "Multi Line Title")]
    #[case("Title\nSubtitle", "Title Subtitle")]
    #[case("Single Line", "Single Line")]
    #[case("  Multiple   Spaces  ", "Multiple Spaces")]
    #[case("\n\nLeading newlines", "Leading newlines")]
    #[case("Trailing newlines\n\n", "Trailing newlines")]
    #[case("Mixed\n  spaces\n\nand\nnewlines", "Mixed spaces and newlines")]
    #[case("Title<br>With<br>Break", "Title With Break")]
    #[case("Mixed<br>breaks\tand\nnewlines", "Mixed breaks and newlines")]
    #[case("<br>Leading break", "Leading break")]
    #[case("Trailing break<br>", "Trailing break")]
    fn test_normalize_heading_content(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(normalize_heading_content(input), expected);
    }

    #[test]
    fn test_normalize_heading_content_cow_efficiency() {
        let input = "Simple text";
        let result = normalize_heading_content(input);
        assert!(matches!(result, Cow::Borrowed(_)));

        let input = "Multi\nLine\nText";
        let result = normalize_heading_content(input);
        assert!(matches!(result, Cow::Owned(_)));
    }

    #[test]
    fn test_cow_to_string_helper() {
        let borrowed = Cow::Borrowed("test");
        assert_eq!(cow_to_string(borrowed), "test");

        let owned = Cow::Owned("test".to_string());
        assert_eq!(cow_to_string(owned), "test");
    }
}
