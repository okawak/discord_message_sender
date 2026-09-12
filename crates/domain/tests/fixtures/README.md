# Compatibility fixtures

`compatibility.json` records outputs from the TypeScript implementations before the Rust migration, based on main commit `cbb6e01` (plugin 0.4.0). All values are synthetic test data.

The cases cover malformed and legacy settings, ECMAScript whitespace, Unicode paths, duplicate names, all aggregated log modes and author/time flags, CRLF/frontmatter/BOM, reserved markers, DST transitions, leap day and ISO week-year boundaries.

`tests/compatibility.rs` compares native Rust output against these fixtures through the crate's public API. Do not regenerate the expected values from the implementation under test.
