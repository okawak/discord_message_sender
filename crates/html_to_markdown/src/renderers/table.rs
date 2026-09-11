use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct Table;

struct TableRow {
    cells: Vec<String>,
    is_header: bool,
}

impl Table {
    fn collect_rows(dom: &Dom, id: NodeId, in_header: bool, rows: &mut Vec<(NodeId, bool)>) {
        let Some(node) = dom.node(id) else {
            return;
        };

        for &child_id in &node.children {
            let Some(child) = dom.node(child_id) else {
                continue;
            };
            let NodeData::Element { tag, .. } = &child.data else {
                continue;
            };

            match tag.local.as_ref() {
                "tr" => rows.push((child_id, in_header)),
                "thead" => Self::collect_rows(dom, child_id, true, rows),
                "tbody" | "tfoot" => Self::collect_rows(dom, child_id, false, rows),
                // Nested tables belong to their containing cell, not this table.
                "table" => {}
                _ => {}
            }
        }
    }

    fn escape_cell_pipes(content: &str) -> String {
        let mut escaped = String::with_capacity(content.len());
        let mut consecutive_backslashes = 0;

        for character in content.chars() {
            match character {
                '\\' => consecutive_backslashes += 1,
                '|' => {
                    for _ in 0..consecutive_backslashes {
                        escaped.push_str("\\\\");
                    }
                    escaped.push('\\');
                    escaped.push(character);
                    consecutive_backslashes = 0;
                }
                _ => {
                    for _ in 0..consecutive_backslashes {
                        escaped.push('\\');
                    }
                    escaped.push(character);
                    consecutive_backslashes = 0;
                }
            }
        }

        for _ in 0..consecutive_backslashes {
            escaped.push('\\');
        }

        escaped
    }

    fn format_cell(content: &str) -> String {
        let flattened = content
            .split(['\r', '\n'])
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("<br>");

        Self::escape_cell_pipes(&flattened)
    }

    fn render_row(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        in_header: bool,
        ctx: &Context,
    ) -> Result<Option<TableRow>, ConvertError> {
        let node = dom.get_node(id)?;
        let mut cells = Vec::new();
        let mut has_header_cell = false;

        for &child_id in &node.children {
            let Some(NodeData::Element { tag, .. }) = dom.node(child_id).map(|node| &node.data)
            else {
                continue;
            };
            if !matches!(tag.local.as_ref(), "th" | "td") {
                continue;
            }

            has_header_cell |= tag.local.as_ref() == "th";
            let mut cell_context = ctx.clone();
            let content = render_children(url, dom, child_id, &mut cell_context)?;
            cells.push(Self::format_cell(&content));
        }

        Ok((!cells.is_empty()).then_some(TableRow {
            cells,
            is_header: in_header || has_header_cell,
        }))
    }

    fn write_row(output: &mut String, cells: &[String], column_count: usize) {
        output.push_str("| ");
        for column in 0..column_count {
            if column > 0 {
                output.push_str(" | ");
            }
            if let Some(cell) = cells.get(column) {
                output.push_str(cell);
            }
        }
        output.push_str(" |\n");
    }

    fn render_caption(
        &self,
        url: &str,
        dom: &Dom,
        table_id: NodeId,
        ctx: &Context,
    ) -> Result<String, ConvertError> {
        let mut captions = Vec::new();
        for &child_id in &dom.get_node(table_id)?.children {
            let Some(NodeData::Element { tag, .. }) = dom.node(child_id).map(|node| &node.data)
            else {
                continue;
            };
            if tag.local.as_ref() != "caption" {
                continue;
            }

            let mut caption_context = ctx.clone();
            let content = render_children(url, dom, child_id, &mut caption_context)?;
            if !content.trim().is_empty() {
                captions.push(content.trim().to_string());
            }
        }
        Ok(captions.join("\n\n"))
    }

    fn render_table(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        ctx: &Context,
    ) -> Result<String, ConvertError> {
        let caption = self.render_caption(url, dom, id, ctx)?;
        let mut row_ids = Vec::new();
        Self::collect_rows(dom, id, false, &mut row_ids);

        let mut rows = Vec::with_capacity(row_ids.len());
        for (row_id, in_header) in row_ids {
            if let Some(row) = self.render_row(url, dom, row_id, in_header, ctx)? {
                rows.push(row);
            }
        }

        let Some(column_count) = rows.iter().map(|row| row.cells.len()).max() else {
            return if caption.is_empty() {
                Ok(String::new())
            } else {
                Ok(format!("{caption}\n\n"))
            };
        };

        let mut output = String::new();
        if !caption.is_empty() {
            output.push_str(&caption);
            output.push_str("\n\n");
        }

        let first_row_is_header = rows.first().is_some_and(|row| row.is_header);
        if first_row_is_header {
            Self::write_row(&mut output, &rows[0].cells, column_count);
        } else {
            Self::write_row(&mut output, &[], column_count);
        }
        let delimiter_cells = vec!["---".to_string(); column_count];
        Self::write_row(&mut output, &delimiter_cells, column_count);

        for row in rows.iter().skip(usize::from(first_row_is_header)) {
            Self::write_row(&mut output, &row.cells, column_count);
        }
        output.push('\n');
        Ok(output)
    }
}

impl Renderer for Table {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        let Some(node) = dom.node(id) else {
            return false;
        };

        if let NodeData::Element { tag, .. } = &node.data {
            matches!(
                tag.local.as_ref(),
                "table" | "thead" | "tbody" | "tr" | "th" | "td"
            )
        } else {
            false
        }
    }

    fn render(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        ctx: &mut Context,
    ) -> Result<String, ConvertError> {
        let (tag, _) = dom.get_element_data(id)?;
        let previous_in_table = ctx.in_table;
        ctx.in_table = true;

        let result = if tag.local.as_ref() == "table" {
            self.render_table(url, dom, id, ctx)
        } else {
            render_children(url, dom, id, ctx)
        };

        ctx.in_table = previous_in_table;
        result
    }
}

pub static TABLE: Table = Table;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parser, renderers};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    fn render(html: &str) -> String {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        renderers::render_node(
            "https://example.com",
            &dom,
            dom.document,
            &mut Context::default(),
        )
        .expect("Failed to render table")
    }

    #[rstest]
    #[case(
        "<table><thead><tr><th>A</th><th>B</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table>",
        "| A | B |\n| --- | --- |\n| 1 | 2 |\n\n"
    )]
    #[case(
        "<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>",
        "| A | B |\n| --- | --- |\n| 1 | 2 |\n\n"
    )]
    #[case(
        "<table><tr><td>1</td><td>2</td></tr><tr><td>3</td><td>4</td></tr></table>",
        "|  |  |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |\n\n"
    )]
    fn renders_gfm_table_structure(#[case] html: &str, #[case] expected: &str) {
        assert_eq!(render(html), expected);
    }

    #[test]
    fn pads_ragged_rows_to_the_widest_row() {
        let html = "<table><tr><th>A</th></tr><tr><td>1</td><td>2</td><td>3</td></tr><tr><td>4</td><td>5</td></tr></table>";

        assert_eq!(
            render(html),
            "| A |  |  |\n| --- | --- | --- |\n| 1 | 2 | 3 |\n| 4 | 5 |  |\n\n"
        );
    }

    #[test]
    fn escapes_cell_pipes_and_flattens_block_line_breaks() {
        let html = r#"<table><tr><th>A|B</th><th>Plain</th></tr><tr><td><p>One</p><p>Two | Three</p></td><td>Last<br>Line</td></tr></table>"#;

        assert_eq!(
            render(html),
            "| A\\|B | Plain |\n| --- | --- |\n| One<br>Two \\| Three | Last<br>Line |\n\n"
        );
        assert_eq!(
            Table::escape_cell_pipes(r#"Already \| escaped"#),
            r#"Already \\\| escaped"#
        );
    }

    #[test]
    fn keeps_caption_separate_from_the_table() {
        let html =
            "<table><caption>Results</caption><tr><th>A</th></tr><tr><td>1</td></tr></table>";

        assert_eq!(render(html), "Results\n\n| A |\n| --- |\n| 1 |\n\n");
    }

    #[test]
    fn restores_the_surrounding_table_context() {
        let dom = parser::parse_html("<table><tr><td>1</td></tr></table>").unwrap();
        let table = dom
            .find_element_by_tag(dom.document, "table")
            .expect("table should exist");
        let mut context = Context {
            in_table: true,
            ..Context::default()
        };

        TABLE
            .render("https://example.com", &dom, table, &mut context)
            .unwrap();

        assert!(context.in_table);
    }
}
