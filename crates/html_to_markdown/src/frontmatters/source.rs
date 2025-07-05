use crate::dom::Dom;
use crate::frontmatters::FrontMatter;

pub struct SourceExtractor;

impl FrontMatter for SourceExtractor {
    fn key(&self) -> &'static str {
        "source"
    }

    fn extract(&self, url: &str, _dom: &Dom) -> Option<String> {
        Some(url.to_string())
    }
}

pub static EXTRACTOR: SourceExtractor = SourceExtractor;
