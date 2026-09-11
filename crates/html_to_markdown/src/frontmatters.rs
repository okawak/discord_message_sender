use crate::dom::Dom;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

pub mod source;
pub mod title;
// pub mod date;
// pub mod tags;

pub trait FrontMatter: Sync + Send {
    fn key(&self) -> &'static str;
    fn extract(&self, url: &str, dom: &Dom) -> Option<String>;
}

pub(crate) fn serialize_yaml_string(value: &str) -> String {
    let mut serialized = String::with_capacity(value.len() + 2);
    serialized.push('"');

    for character in value.chars() {
        match character {
            '"' => serialized.push_str("\\\""),
            '\\' => serialized.push_str("\\\\"),
            '\u{0000}'..='\u{001f}'
            | '\u{007f}'..='\u{009f}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{fffe}'
            | '\u{ffff}' => {
                use std::fmt::Write;
                write!(serialized, "\\u{:04X}", character as u32)
                    .expect("writing to a String cannot fail");
            }
            _ => serialized.push(character),
        }
    }

    serialized.push('"');
    serialized
}

static FRONTMATTER_MAP: LazyLock<HashMap<&'static str, &'static dyn FrontMatter>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        map.insert("title", &title::EXTRACTOR as &'static dyn FrontMatter);
        map.insert("source", &source::EXTRACTOR as &'static dyn FrontMatter);
        // map.insert("tags", &tags::EXTRACTOR as &'static dyn FrontMatter);
        // map.insert("date", &date::EXTRACTOR as &'static dyn FrontMatter);
        map
    });

pub fn get_frontmatter_extractors(keys: &[&str]) -> Vec<(&'static str, &'static dyn FrontMatter)> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for &key in keys {
        if seen.insert(key)
            && let Some(&extractor) = FRONTMATTER_MAP.get(key)
        {
            result.push((extractor.key(), extractor));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::serialize_yaml_string;
    use rstest::rstest;

    #[rstest]
    #[case("Title", r#""Title""#)]
    #[case("Mapping: #1", r#""Mapping: #1""#)]
    #[case(" null ", r#"" null ""#)]
    #[case("true", r#""true""#)]
    #[case("123", r#""123""#)]
    #[case(r#""quoted" \ path"#, r#""\"quoted\" \\ path""#)]
    #[case(
        "line\ncarriage\rtab\t\0\u{001f}\u{007f}\u{0085}\u{2028}\u{2029}\u{fffe}\u{ffff}",
        r#""line\u000Acarriage\u000Dtab\u0009\u0000\u001F\u007F\u0085\u2028\u2029\uFFFE\uFFFF""#
    )]
    fn serializes_as_a_single_yaml_string(#[case] value: &str, #[case] expected: &str) {
        assert_eq!(serialize_yaml_string(value), expected);
    }
}
