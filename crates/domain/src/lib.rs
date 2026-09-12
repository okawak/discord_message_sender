//! Obsidian-independent domain logic. JavaScript owns host I/O and object identity.

pub mod channels;
mod command;
pub mod dates;
pub mod discord;
pub mod logs;
pub mod messages;
pub mod models;
pub mod settings;
pub mod storage;
pub mod sync;

/// ECMAScript trim/whitespace, kept explicit for existing settings compatibility.
pub fn js_whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
pub fn trim(value: &str) -> &str {
    value.trim_matches(js_whitespace)
}
