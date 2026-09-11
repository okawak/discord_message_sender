use super::{Context, Renderer, is_block_element, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct Table;

struct TableRow {
    cells: Vec<String>,
    is_header: bool,
}

struct TableRowGroup {
    row_ids: Vec<NodeId>,
    is_header: bool,
}

const MAX_COLSPAN: usize = 1_000;

impl Table {
    fn collect_row_groups(dom: &Dom, table_id: NodeId) -> Vec<TableRowGroup> {
        let mut groups = Vec::new();
        let mut implicit_rows = Vec::new();
        let Some(table) = dom.node(table_id) else {
            return groups;
        };

        for &child_id in &table.children {
            let Some(child) = dom.node(child_id) else {
                continue;
            };
            let NodeData::Element { tag, .. } = &child.data else {
                continue;
            };

            match tag.local.as_ref() {
                "tr" => implicit_rows.push(child_id),
                "thead" | "tbody" | "tfoot" => {
                    if !implicit_rows.is_empty() {
                        groups.push(TableRowGroup {
                            row_ids: std::mem::take(&mut implicit_rows),
                            is_header: false,
                        });
                    }

                    let row_ids = child
                        .children
                        .iter()
                        .copied()
                        .filter(|&row_id| {
                            matches!(
                                dom.node(row_id).map(|row| &row.data),
                                Some(NodeData::Element { tag, .. }) if tag.local.as_ref() == "tr"
                            )
                        })
                        .collect::<Vec<_>>();
                    if !row_ids.is_empty() {
                        groups.push(TableRowGroup {
                            row_ids,
                            is_header: tag.local.as_ref() == "thead",
                        });
                    }
                }
                _ => {}
            }
        }

        if !implicit_rows.is_empty() {
            groups.push(TableRowGroup {
                row_ids: implicit_rows,
                is_header: false,
            });
        }

        groups
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

    fn span(attrs: &std::collections::HashMap<String, String>, name: &str) -> Option<usize> {
        attrs.get(name)?.trim().parse().ok()
    }

    fn next_available_column(active_rowspans: &[usize], start: usize, width: usize) -> usize {
        let mut column = start;
        loop {
            let blocked = (column..column + width)
                .find(|&candidate| active_rowspans.get(candidate).copied().unwrap_or(0) > 0);
            match blocked {
                Some(blocked_column) => column = blocked_column + 1,
                None => return column,
            }
        }
    }

    fn is_definitely_empty(dom: &Dom, id: NodeId) -> bool {
        let Some(node) = dom.node(id) else {
            return true;
        };

        match &node.data {
            NodeData::Text(text) => text.trim().is_empty(),
            NodeData::Element { tag, .. } => {
                let tag_name = tag.local.as_ref();
                if matches!(tag_name, "script" | "style" | "noscript" | "footer" | "nav") {
                    return true;
                }

                matches!(
                    tag_name,
                    "p" | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "div"
                        | "section"
                        | "summary"
                        | "article"
                        | "main"
                        | "header"
                        | "span"
                        | "del"
                        | "ins"
                        | "mark"
                        | "sub"
                        | "sup"
                        | "small"
                ) && node
                    .children
                    .iter()
                    .all(|&child_id| Self::is_definitely_empty(dom, child_id))
            }
            _ => true,
        }
    }

    fn has_preceding_unseparated_content(dom: &Dom, id: NodeId) -> bool {
        let Ok(Some(parent_id)) = dom.get_parent(id) else {
            return false;
        };
        let Ok(children) = dom.iter_children(parent_id) else {
            return false;
        };
        let children = children.copied().collect::<Vec<_>>();
        let Some(index) = children.iter().position(|&candidate| candidate == id) else {
            return false;
        };

        for &sibling_id in children[..index].iter().rev() {
            let Some(sibling) = dom.node(sibling_id) else {
                continue;
            };
            if Self::is_definitely_empty(dom, sibling_id) {
                continue;
            }
            match &sibling.data {
                NodeData::Text(_) => return true,
                NodeData::Element { tag, .. } => {
                    let tag_name = tag.local.as_ref();
                    // GenericBlock renders div transparently, so its tag alone does not
                    // guarantee a trailing Markdown block boundary.
                    return tag_name == "div" || !is_block_element(tag_name);
                }
                _ => continue,
            }
        }

        false
    }

    fn render_row(
        url: &str,
        dom: &Dom,
        id: NodeId,
        in_header: bool,
        ctx: &Context,
        remaining_rows: usize,
        active_rowspans: &mut Vec<usize>,
    ) -> Result<Option<TableRow>, ConvertError> {
        let node = dom.get_node(id)?;
        let had_active_rowspan = active_rowspans.iter().any(|&remaining| remaining > 0);
        let mut cells = vec![String::new(); active_rowspans.len()];
        let mut next_column = 0;
        let mut source_cell_count = 0;
        let mut all_cells_are_column_headers = true;
        let mut has_explicit_column_header = false;

        for &child_id in &node.children {
            let Some(NodeData::Element { tag, attrs }) = dom.node(child_id).map(|node| &node.data)
            else {
                continue;
            };
            if !matches!(tag.local.as_ref(), "th" | "td") {
                continue;
            }

            source_cell_count += 1;
            let scope = attrs.get("scope").map(String::as_str).unwrap_or_default();
            let is_column_header = tag.local.as_ref() == "th"
                && !matches!(scope.to_ascii_lowercase().as_str(), "row" | "rowgroup");
            all_cells_are_column_headers &= is_column_header;
            has_explicit_column_header |= tag.local.as_ref() == "th"
                && matches!(scope.to_ascii_lowercase().as_str(), "col" | "colgroup");

            let colspan = Self::span(attrs, "colspan")
                .unwrap_or(1)
                .clamp(1, MAX_COLSPAN);
            let rowspan = match Self::span(attrs, "rowspan") {
                Some(0) => remaining_rows,
                Some(value) => value.clamp(1, remaining_rows),
                None => 1,
            };
            let column = Self::next_available_column(active_rowspans, next_column, colspan);
            let end_column = column + colspan;
            if cells.len() < end_column {
                cells.resize(end_column, String::new());
            }
            if active_rowspans.len() < end_column {
                active_rowspans.resize(end_column, 0);
            }

            let mut cell_context = ctx.clone();
            let content = render_children(url, dom, child_id, &mut cell_context)?;
            cells[column] = Self::format_cell(&content);
            for active in &mut active_rowspans[column..end_column] {
                *active = (*active).max(rowspan);
            }
            next_column = end_column;
        }

        for active in active_rowspans.iter_mut() {
            *active = active.saturating_sub(1);
        }

        Ok(
            (source_cell_count > 0 || had_active_rowspan).then_some(TableRow {
                cells,
                is_header: in_header
                    || has_explicit_column_header
                    || (source_cell_count > 0 && all_cells_are_column_headers),
            }),
        )
    }

    fn write_row(output: &mut String, indent: &str, cells: &[String], column_count: usize) {
        output.push_str(indent);
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
        let row_groups = Self::collect_row_groups(dom, id);

        let list_indent = " ".repeat(ctx.list_depth);
        let starts_after_list_content = ctx.list_depth > 0 && !ctx.list_first_item;
        let needs_leading_boundary = starts_after_list_content
            || (ctx.list_depth == 0 && Self::has_preceding_unseparated_content(dom, id));
        let first_line_indent = if starts_after_list_content {
            list_indent.as_str()
        } else {
            ""
        };
        let continuation_indent = if ctx.list_depth > 0 {
            list_indent.as_str()
        } else {
            ""
        };

        let row_capacity = row_groups.iter().map(|group| group.row_ids.len()).sum();
        let mut rows = Vec::with_capacity(row_capacity);
        for group in row_groups {
            // HTML row spans are scoped to their row group and cannot shift cells in a
            // following thead, tbody, or tfoot.
            let mut active_rowspans = Vec::new();
            let row_count = group.row_ids.len();
            for (index, row_id) in group.row_ids.into_iter().enumerate() {
                if let Some(row) = Self::render_row(
                    url,
                    dom,
                    row_id,
                    group.is_header,
                    ctx,
                    row_count - index,
                    &mut active_rowspans,
                )? {
                    rows.push(row);
                }
            }
        }

        let Some(column_count) = rows.iter().map(|row| row.cells.len()).max() else {
            return if caption.is_empty() {
                Ok(String::new())
            } else {
                let mut output = String::new();
                if needs_leading_boundary {
                    output.push_str("\n\n");
                }
                output.push_str(first_line_indent);
                output.push_str(&caption);
                output.push_str("\n\n");
                Ok(output)
            };
        };

        let mut output = String::new();
        if needs_leading_boundary {
            output.push_str("\n\n");
        }
        if !caption.is_empty() {
            output.push_str(first_line_indent);
            output.push_str(&caption);
            output.push_str("\n\n");
            output.push_str(continuation_indent);
        }

        let table_first_line_indent = if caption.is_empty() {
            first_line_indent
        } else {
            ""
        };
        let first_row_is_header = rows.first().is_some_and(|row| row.is_header);
        if first_row_is_header {
            Self::write_row(
                &mut output,
                table_first_line_indent,
                &rows[0].cells,
                column_count,
            );
        } else {
            Self::write_row(&mut output, table_first_line_indent, &[], column_count);
        }
        let delimiter_cells = vec!["---".to_string(); column_count];
        Self::write_row(
            &mut output,
            continuation_indent,
            &delimiter_cells,
            column_count,
        );

        for row in rows.iter().skip(usize::from(first_row_is_header)) {
            Self::write_row(&mut output, continuation_indent, &row.cells, column_count);
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
    fn keeps_row_headers_in_the_table_body() {
        let html = r#"<table><tr><th scope="row">A</th><td>1</td></tr><tr><th scope="row">B</th><td>2</td></tr></table>"#;

        assert_eq!(
            render(html),
            "|  |  |\n| --- | --- |\n| A | 1 |\n| B | 2 |\n\n"
        );
    }

    #[test]
    fn uses_explicit_column_headers_in_a_mixed_row() {
        let html = r#"<table><tr><th scope="col">Name</th><td>Value</td></tr><tr><td>A</td><td>1</td></tr></table>"#;

        assert_eq!(
            render(html),
            "| Name | Value |\n| --- | --- |\n| A | 1 |\n\n"
        );
    }

    #[test]
    fn reserves_columns_occupied_by_rowspan_and_colspan() {
        let html = r#"<table><tr><td rowspan="2">A</td><td>B</td></tr><tr><td>C</td></tr><tr><td colspan="2">D</td></tr></table>"#;

        assert_eq!(
            render(html),
            "|  |  |\n| --- | --- |\n| A | B |\n|  | C |\n| D |  |\n\n"
        );
    }

    #[rstest]
    #[case(
        r#"<table><thead><tr><th rowspan="2">A</th><th>B</th></tr></thead><tbody><tr><td>C</td><td>D</td></tr></tbody></table>"#,
        "| A | B |\n| --- | --- |\n| C | D |\n\n"
    )]
    #[case(
        r#"<table><tbody><tr><td rowspan="0">A</td><td>B</td></tr><tr><td>C</td></tr></tbody><tbody><tr><td>D</td><td>E</td></tr></tbody></table>"#,
        "|  |  |\n| --- | --- |\n| A | B |\n|  | C |\n| D | E |\n\n"
    )]
    fn limits_rowspans_to_their_row_group(#[case] html: &str, #[case] expected: &str) {
        assert_eq!(render(html), expected);
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

    #[rstest]
    #[case(
        "<div>Before<table><tr><th>A</th></tr><tr><td>1</td></tr></table></div>",
        "Before\n\n| A |\n| --- |\n| 1 |\n\n"
    )]
    #[case(
        "<div>Before</div><table><tr><th>A</th></tr><tr><td>1</td></tr></table>",
        "Before\n\n| A |\n| --- |\n| 1 |\n\n"
    )]
    #[case(
        "Before<table><caption>Results</caption></table>",
        "Before\n\nResults\n\n"
    )]
    #[case(
        "Before<p></p><table><tr><th>A</th></tr><tr><td>1</td></tr></table>",
        "Before\n\n| A |\n| --- |\n| 1 |\n\n"
    )]
    #[case(
        "Before<nav>Ignored</nav><div><span></span></div><table><tr><th>A</th></tr></table>",
        "Before\n\n| A |\n| --- |\n\n"
    )]
    #[case(
        "<ul><li><table><tr><th>A</th></tr><tr><td>1</td></tr></table></li></ul>",
        "- | A |\n  | --- |\n  | 1 |\n\n"
    )]
    #[case(
        "<ol><li>Before<table><tr><th>A</th></tr><tr><td>1</td></tr></table></li></ol>",
        "1. Before\n\n   | A |\n   | --- |\n   | 1 |\n\n"
    )]
    fn keeps_tables_on_valid_block_boundaries(#[case] html: &str, #[case] expected: &str) {
        assert_eq!(render(html), expected);
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
