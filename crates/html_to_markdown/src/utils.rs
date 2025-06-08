pub fn normalize_html_text(text: &str) -> Option<String> {
    // Early return for empty strings
    if text.trim().is_empty() {
        return None;
    }

    let normalized = text
        .chars()
        .filter(|c| {
            // Keep control characters that represent whitespace (tab, newline, return)
            if c.is_control() && !matches!(*c, '\t' | '\n' | '\r') {
                return false;
            }

            // Remove invisible and zero-width characters
            match *c {
                '\u{00A0}' |  // Non-breaking space
                '\u{200B}' |  // Zero-width space
                '\u{200C}' |  // Zero-width non-joiner
                '\u{200D}' |  // Zero-width joiner
                '\u{2060}' |  // Word joiner
                '\u{FEFF}'    // Zero-width non-breaking space (BOM)
                => false,
                _ => true
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::*;

    #[rstest]
    #[case("", None)]
    #[case("   ", None)]
    #[case("Hello World", Some("Hello World"))]
    #[case("  Hello   World  ", Some("Hello World"))]
    #[case("\u{00A0}\u{200B}Hello\u{FEFF}World\u{2060}", Some("HelloWorld"))]
    #[case("Multi\nLine\tText", Some("Multi Line Text"))]
    fn test_normalize_html_text(#[case] input: &str, #[case] expected: Option<&str>) {
        assert_eq!(normalize_html_text(input), expected.map(String::from));
    }
}
