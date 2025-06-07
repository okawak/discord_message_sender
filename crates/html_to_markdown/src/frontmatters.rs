use crate::dom::Dom;

pub trait FrontMatter: Sync + Send {
    fn key(&self) -> &'static str;
    fn extract(&self, dom: &Dom) -> Option<String>;
}

// pub mod date;
// pub mod tags;
// pub mod title;

pub static FRONTMATTERS: &[&dyn FrontMatter] = &[];
// &[&title::EXTRACTOR, &tags::EXTRACTOR, &date::EXTRACTOR];
