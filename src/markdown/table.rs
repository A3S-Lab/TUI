use comrak::nodes::{AstNode, NodeTable, NodeValue, TableAlignment};

use super::ansi::{ansi_segments, wrap_styled_text};
use super::{fit_markdown_line, Markdown, MarkdownLine};
use crate::style::{visible_len, Color, Style};

#[derive(Debug)]
struct RichTable {
    alignments: Vec<TableAlignment>,
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Markdown {
    pub(super) fn render_table<'a>(
        &self,
        node: &'a AstNode<'a>,
        metadata: &NodeTable,
        output: &mut Vec<MarkdownLine>,
        indent: usize,
        width: usize,
    ) {
        let table = self.collect_table(node, metadata);
        let column_count = table.alignments.len();
        if column_count == 0 {
            return;
        }

        let mut natural_widths = vec![0usize; column_count];
        for row in std::iter::once(&table.header).chain(table.rows.iter()) {
            for (column, cell) in row.iter().enumerate() {
                natural_widths[column] = natural_widths[column].max(visible_len(cell));
            }
        }
        let available = if width == 0 {
            usize::MAX
        } else {
            width.saturating_sub(indent)
        };

        if table.rows.is_empty() && table_grid_width(&natural_widths) > available {
            self.render_header_only_table(&table, output, indent, width);
        } else if let Some(widths) = allocate_table_widths(&table, &natural_widths, available) {
            self.render_table_grid(&table, &widths, output, indent);
        } else {
            self.render_table_records(&table, output, indent, width);
        }
    }

    fn collect_table<'a>(&self, node: &'a AstNode<'a>, metadata: &NodeTable) -> RichTable {
        let mut header = None;
        let mut rows = Vec::new();
        let mut column_count = metadata.num_columns.max(metadata.alignments.len());

        for row in node.children() {
            let NodeValue::TableRow(is_header) = &row.data.borrow().value else {
                continue;
            };
            let cells = row
                .children()
                .map(|cell| {
                    self.collect_inline(cell)
                        .replace('\n', " ")
                        .trim()
                        .to_string()
                })
                .collect::<Vec<_>>();
            column_count = column_count.max(cells.len());
            if *is_header && header.is_none() {
                header = Some(cells);
            } else {
                rows.push(cells);
            }
        }

        let mut alignments = metadata.alignments.clone();
        alignments.resize(column_count, TableAlignment::None);
        let mut header = header.unwrap_or_default();
        header.resize(column_count, String::new());
        for row in &mut rows {
            row.resize(column_count, String::new());
        }
        RichTable {
            alignments,
            header,
            rows,
        }
    }

    fn render_table_grid(
        &self,
        table: &RichTable,
        widths: &[usize],
        output: &mut Vec<MarkdownLine>,
        indent: usize,
    ) {
        const COLUMN_GAP: &str = "  ";
        let indent_text = " ".repeat(indent);
        let separator = |fill: char| {
            let line = widths
                .iter()
                .map(|width| fill.to_string().repeat(width.saturating_add(2)))
                .collect::<Vec<_>>()
                .join(COLUMN_GAP);
            format!(
                "{indent_text}{}",
                Style::new().fg(Color::BrightBlack).render(&line)
            )
        };
        output.extend(
            render_table_row(&table.header, widths, &table.alignments, indent, true)
                .into_iter()
                .map(MarkdownLine::normal),
        );
        output.push(MarkdownLine::normal(separator('━')));
        for (row_index, body) in table.rows.iter().enumerate() {
            output.extend(
                render_table_row(body, widths, &table.alignments, indent, false)
                    .into_iter()
                    .map(MarkdownLine::normal),
            );
            if row_index + 1 < table.rows.len() {
                output.push(MarkdownLine::normal(separator('─')));
            }
        }
    }

    fn render_table_records(
        &self,
        table: &RichTable,
        output: &mut Vec<MarkdownLine>,
        indent: usize,
        width: usize,
    ) {
        for (row_index, row) in table.rows.iter().enumerate() {
            if row_index > 0 {
                output.push(MarkdownLine::normal(String::new()));
            }
            for column in 0..table.alignments.len() {
                let label = table.header[column].as_str();
                let fallback;
                let label = if label.is_empty() {
                    fallback = format!("column {}", column + 1);
                    fallback.as_str()
                } else {
                    label
                };
                let cell = row.get(column).map(String::as_str).unwrap_or("");
                self.push_table_record(output, indent, width, label, cell);
            }
        }
    }

    fn push_table_record(
        &self,
        output: &mut Vec<MarkdownLine>,
        indent: usize,
        width: usize,
        label: &str,
        cell: &str,
    ) {
        let label_width = visible_len(label);
        let prefix_width = indent.saturating_add(label_width).saturating_add(2);
        let styled_label = format!("\x1b[1;90m{label}\x1b[22;39m:");
        if width != 0 && prefix_width >= width {
            let label_indent_width = indent.min(width.saturating_sub(1));
            let label_indent = " ".repeat(label_indent_width);
            let label_width = width.saturating_sub(label_indent_width).max(1);
            output.extend(
                wrap_styled_text(&styled_label, label_width)
                    .into_iter()
                    .map(|line| {
                        MarkdownLine::normal(fit_markdown_line(
                            format!("{label_indent}{line}"),
                            width,
                        ))
                    }),
            );

            if !cell.is_empty() {
                let cell_indent_width = label_indent_width
                    .saturating_add(1)
                    .min(width.saturating_sub(1));
                let cell_indent = " ".repeat(cell_indent_width);
                let cell_width = width.saturating_sub(cell_indent_width).max(1);
                output.extend(wrap_styled_text(cell, cell_width).into_iter().map(|line| {
                    MarkdownLine::normal(fit_markdown_line(format!("{cell_indent}{line}"), width))
                }));
            }
            return;
        }

        let content_width = if width == 0 {
            0
        } else {
            width.saturating_sub(prefix_width).max(1)
        };
        let wrapped = wrap_styled_text(cell, content_width);
        let label = format!("{styled_label} ");
        let first_prefix = format!("{}{label}", " ".repeat(indent));
        let continuation = " ".repeat(prefix_width);

        for (line_index, line) in wrapped.into_iter().enumerate() {
            let prefix = if line_index == 0 {
                first_prefix.as_str()
            } else {
                continuation.as_str()
            };
            output.push(MarkdownLine::normal(format!("{prefix}{line}")));
        }
    }

    fn render_header_only_table(
        &self,
        table: &RichTable,
        output: &mut Vec<MarkdownLine>,
        indent: usize,
        width: usize,
    ) {
        let available = if width == 0 {
            0
        } else {
            width.saturating_sub(indent).max(1)
        };
        let indent = " ".repeat(indent);
        for cell in table.header.iter().filter(|cell| !cell.is_empty()) {
            let styled = format!("\x1b[1m{cell}\x1b[22m");
            for wrapped in wrap_styled_text(&styled, available) {
                output.push(MarkdownLine::normal(format!("{indent}{wrapped}")));
            }
        }
    }
}
fn table_grid_width(widths: &[usize]) -> usize {
    widths
        .iter()
        .sum::<usize>()
        .saturating_add(widths.len().saturating_mul(2))
        .saturating_add(widths.len().saturating_sub(1).saturating_mul(2))
}

#[derive(Clone, Copy)]
struct TableColumnMetric {
    floor: usize,
    shrink_priority: usize,
}

fn allocate_table_widths(
    table: &RichTable,
    natural: &[usize],
    available: usize,
) -> Option<Vec<usize>> {
    if natural.is_empty() {
        return Some(Vec::new());
    }
    if table_grid_width(natural) <= available {
        return Some(natural.to_vec());
    }

    let overhead = table_grid_width(&vec![0; natural.len()]);
    let content_budget = available.checked_sub(overhead)?;
    if natural.len() >= 4 && content_budget / natural.len() < 4 {
        return None;
    }

    let metrics = (0..natural.len())
        .map(|column| {
            let mut longest_word = 0usize;
            let mut has_whitespace = false;
            for row in std::iter::once(&table.header).chain(table.rows.iter()) {
                let cell = row.get(column).map(String::as_str).unwrap_or("");
                let plain = ansi_segments(cell)
                    .into_iter()
                    .map(|segment| segment.text)
                    .collect::<String>();
                has_whitespace |= plain.chars().any(char::is_whitespace);
                longest_word =
                    longest_word.max(plain.split_whitespace().map(visible_len).max().unwrap_or(0));
            }
            let width = natural[column];
            if width <= 8 {
                TableColumnMetric {
                    floor: width,
                    shrink_priority: 2,
                }
            } else if !has_whitespace || longest_word >= 12 {
                TableColumnMetric {
                    floor: width.min(6),
                    shrink_priority: 0,
                }
            } else {
                TableColumnMetric {
                    floor: width.min(8),
                    shrink_priority: 1,
                }
            }
        })
        .collect::<Vec<_>>();
    if metrics.iter().map(|metric| metric.floor).sum::<usize>() > content_budget {
        return None;
    }

    let mut widths = natural.to_vec();
    while widths.iter().sum::<usize>() > content_budget {
        let (column, _) = widths
            .iter()
            .enumerate()
            .filter(|(column, width)| **width > metrics[*column].floor)
            .min_by_key(|(column, width)| {
                let slack = width.saturating_sub(metrics[*column].floor);
                (
                    metrics[*column].shrink_priority,
                    usize::MAX.saturating_sub(slack),
                )
            })?;
        widths[column] -= 1;
    }
    Some(widths)
}

fn render_table_row(
    cells: &[String],
    widths: &[usize],
    alignments: &[TableAlignment],
    indent: usize,
    header: bool,
) -> Vec<String> {
    const COLUMN_GAP: &str = "  ";
    let wrapped = widths
        .iter()
        .enumerate()
        .map(|(column, width)| {
            wrap_styled_text(cells.get(column).map(String::as_str).unwrap_or(""), *width)
        })
        .collect::<Vec<_>>();
    let height = wrapped.iter().map(Vec::len).max().unwrap_or(1);
    let mut rows = Vec::with_capacity(height);
    for row_index in 0..height {
        let mut row = " ".repeat(indent);
        for (column, width) in widths.iter().enumerate() {
            if column > 0 {
                row.push_str(COLUMN_GAP);
            }
            let cell = wrapped[column]
                .get(row_index)
                .map(String::as_str)
                .unwrap_or("");
            let remaining = width.saturating_sub(visible_len(cell));
            let (left_padding, right_padding) = match alignments[column] {
                TableAlignment::None | TableAlignment::Left => (0, remaining),
                TableAlignment::Center => (remaining / 2, remaining - (remaining / 2)),
                TableAlignment::Right => (remaining, 0),
            };
            row.push(' ');
            row.push_str(&" ".repeat(left_padding));
            if header {
                row.push_str(&format!("\x1b[1m{cell}\x1b[22m"));
            } else {
                row.push_str(cell);
            }
            row.push_str(&" ".repeat(right_padding));
            row.push(' ');
        }
        rows.push(row);
    }
    rows
}
