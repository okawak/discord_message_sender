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
