use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_CONNECTOR_BLOCK_GAP: usize = u16::MAX as usize;
const MAX_CONNECTOR_BLOCK_INDENT: usize = u16::MAX as usize;
const MAX_CONNECTOR_BLOCK_MARGIN: usize = u16::MAX as usize;
const MAX_CONNECTOR_BLOCK_ROWS: usize = u16::MAX as usize;

/// Connector-led rows for compact tool output and task summaries.
///
/// This extracts the CLI pattern used below completed tool calls: the first row
/// starts with a muted connector such as `⎿`, while continuation rows align
/// under the body text. Rows are display-width safe and can compact noisy output
/// to the latest lines with an omitted-count marker.
#[derive(Debug, Clone)]
pub struct ConnectorBlock {
    rows: Vec<ConnectorRow>,
    max_rows: Option<usize>,
    show_omitted_count: bool,
    margin: usize,
    connector: String,
    connector_indent: usize,
    connector_gap: usize,
    repeat_connector: bool,
    text_color: Color,
    omitted_color: Color,
    connector_color: Color,
}

impl ConnectorBlock {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            max_rows: None,
            show_omitted_count: true,
            margin: 2,
            connector: "⎿".to_string(),
            connector_indent: 2,
            connector_gap: 2,
            repeat_connector: false,
            text_color: Color::BrightBlack,
            omitted_color: Color::BrightBlack,
            connector_color: Color::BrightBlack,
        }
    }

    pub fn lines(lines: Vec<impl Into<String>>) -> Self {
        Self::new().rows(
            lines
                .into_iter()
                .map(|line| ConnectorRow::new(line.into()))
                .collect(),
        )
    }

    pub fn text(text: impl AsRef<str>) -> Self {
        Self::new().rows(
            text.as_ref()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| ConnectorRow::new(line.to_string()))
                .collect(),
        )
    }

    pub fn row(mut self, row: ConnectorRow) -> Self {
        self.rows.push(row);
        self
    }

    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.rows.push(ConnectorRow::new(line));
        self
    }

    pub fn rows(mut self, rows: Vec<ConnectorRow>) -> Self {
        self.rows = rows;
        self
    }

    pub fn add_row(&mut self, row: ConnectorRow) {
        self.rows.push(row);
    }

    pub fn add_line(&mut self, line: impl Into<String>) {
        self.rows.push(ConnectorRow::new(line));
    }

    pub fn max_rows(mut self, max_rows: usize) -> Self {
        self.max_rows = Some(max_rows.clamp(1, MAX_CONNECTOR_BLOCK_ROWS));
        self
    }

    pub fn unlimited_rows(mut self) -> Self {
        self.max_rows = None;
        self
    }

    pub fn show_omitted_count(mut self, show: bool) -> Self {
        self.show_omitted_count = show;
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin.min(MAX_CONNECTOR_BLOCK_MARGIN);
        self
    }

    pub fn connector(mut self, connector: impl Into<String>) -> Self {
        let connector = connector.into();
        if !connector.is_empty() {
            self.connector = connector;
        }
        self
    }

    pub fn connector_indent(mut self, indent: usize) -> Self {
        self.connector_indent = indent.min(MAX_CONNECTOR_BLOCK_INDENT);
        self
    }

    pub fn connector_gap(mut self, gap: usize) -> Self {
        self.connector_gap = gap.min(MAX_CONNECTOR_BLOCK_GAP);
        self
    }

    pub fn repeat_connector(mut self, repeat: bool) -> Self {
        self.repeat_connector = repeat;
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    pub fn omitted_color(mut self, color: Color) -> Self {
        self.omitted_color = color;
        self
    }

    pub fn connector_color(mut self, color: Color) -> Self {
        self.connector_color = color;
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.text_color = theme.color(ThemeRole::Muted);
        self.omitted_color = theme.color(ThemeRole::Muted);
        self.connector_color = theme.color(ThemeRole::Border);
        self
    }

    pub fn rows_value(&self) -> &[ConnectorRow] {
        &self.rows
    }

    pub fn view(&self, width: u16) -> String {
        let width = width as usize;
        if width == 0 || self.rows.is_empty() {
            return String::new();
        }

        let body_width = self.body_width(width);
        self.visible_rows()
            .into_iter()
            .enumerate()
            .map(|(index, row)| {
                let prefix = if index == 0 || self.repeat_connector {
                    self.render_first_prefix(width)
                } else {
                    self.continuation_prefix(width)
                };
                let text = truncate_visible(&row.text, body_width);
                let color = row.color.unwrap_or_else(|| self.row_color(&row));
                fit_visible(
                    &format!("{prefix}{}", Style::new().fg(color).render(&text)),
                    width,
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        if self.rows.is_empty() {
            return Element::Box(BoxElement::new().direction(FlexDirection::Column));
        }

        Element::Box(
            BoxElement::new().direction(FlexDirection::Column).children(
                self.visible_rows()
                    .into_iter()
                    .enumerate()
                    .map(|(index, row)| self.row_element(index, row))
                    .collect(),
            ),
        )
    }

    fn visible_rows(&self) -> Vec<ConnectorRow> {
        let Some(max_rows) = self.max_rows else {
            return self.rows.clone();
        };
        if self.rows.len() <= max_rows {
            return self.rows.clone();
        }

        let start = self.rows.len().saturating_sub(max_rows);
        let mut rows =
            Vec::with_capacity(max_rows.saturating_add(usize::from(self.show_omitted_count)));
        if self.show_omitted_count {
            rows.push(ConnectorRow::new(format!("… +{start} earlier lines")).omitted());
        }
        rows.extend(self.rows.iter().skip(start).cloned());
        rows
    }

    fn row_element<Msg>(&self, index: usize, row: ConnectorRow) -> Element<Msg> {
        let mut children = Vec::new();
        if index == 0 || self.repeat_connector {
            children.push(Element::Text(TextElement::new(
                self.first_prefix_margin_for_element(),
            )));
            children.push(Element::Text(
                TextElement::new(self.connector.clone()).fg(self.connector_color),
            ));
            children.push(Element::Text(TextElement::new(
                " ".repeat(self.connector_gap_for_element()),
            )));
        } else {
            children.push(Element::Text(TextElement::new(
                self.continuation_prefix_for_element(),
            )));
        }

        let color = row.color.unwrap_or_else(|| self.row_color(&row));
        children.push(Element::Text(TextElement::new(row.text).fg(color)));

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }

    fn row_color(&self, row: &ConnectorRow) -> Color {
        if row.omitted {
            self.omitted_color
        } else {
            self.text_color
        }
    }

    fn render_first_prefix(&self, width: usize) -> String {
        format!(
            "{}{}{}",
            self.first_prefix_margin(width),
            Style::new()
                .fg(self.connector_color)
                .render(&self.connector),
            " ".repeat(self.connector_gap_for_width(width))
        )
    }

    fn first_prefix_margin(&self, width: usize) -> String {
        format!(
            "{}{}",
            " ".repeat(self.margin_for_width(width)),
            " ".repeat(self.connector_indent_for_width(width))
        )
    }

    fn first_prefix_margin_for_element(&self) -> String {
        format!(
            "{}{}",
            " ".repeat(self.margin_for_element()),
            " ".repeat(self.connector_indent_for_element())
        )
    }

    fn continuation_prefix(&self, width: usize) -> String {
        " ".repeat(self.prefix_width(width))
    }

    fn continuation_prefix_for_element(&self) -> String {
        " ".repeat(self.prefix_width_for_element())
    }

    fn prefix_width(&self, width: usize) -> usize {
        [
            self.margin_for_width(width),
            self.connector_indent_for_width(width),
            visible_len(&self.connector),
            self.connector_gap_for_width(width),
        ]
        .into_iter()
        .fold(0usize, usize::saturating_add)
    }

    fn prefix_width_for_element(&self) -> usize {
        [
            self.margin_for_element(),
            self.connector_indent_for_element(),
            visible_len(&self.connector),
            self.connector_gap_for_element(),
        ]
        .into_iter()
        .fold(0usize, usize::saturating_add)
    }

    fn body_width(&self, width: usize) -> usize {
        width.saturating_sub(self.prefix_width(width)).max(1)
    }

    fn margin_for_width(&self, width: usize) -> usize {
        self.margin.min(width).min(MAX_CONNECTOR_BLOCK_MARGIN)
    }

    fn connector_indent_for_width(&self, width: usize) -> usize {
        self.connector_indent
            .min(width)
            .min(MAX_CONNECTOR_BLOCK_INDENT)
    }

    fn connector_gap_for_width(&self, width: usize) -> usize {
        self.connector_gap.min(width).min(MAX_CONNECTOR_BLOCK_GAP)
    }

    fn margin_for_element(&self) -> usize {
        self.margin.min(MAX_CONNECTOR_BLOCK_MARGIN)
    }

    fn connector_indent_for_element(&self) -> usize {
        self.connector_indent.min(MAX_CONNECTOR_BLOCK_INDENT)
    }

    fn connector_gap_for_element(&self) -> usize {
        self.connector_gap.min(MAX_CONNECTOR_BLOCK_GAP)
    }
}

impl Default for ConnectorBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorRow {
    text: String,
    color: Option<Color>,
    omitted: bool,
}

impl ConnectorRow {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
            omitted: false,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn omitted(mut self) -> Self {
        self.omitted = true;
        self
    }

    pub fn text_value(&self) -> &str {
        &self.text
    }

    pub fn color_value(&self) -> Option<Color> {
        self.color
    }

    pub fn is_omitted(&self) -> bool {
        self.omitted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn renders_first_connector_and_aligned_continuations() {
        let rendered = ConnectorBlock::new()
            .line("Task completed")
            .line("child output stored")
            .view(48);
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("    ⎿  Task completed"), "{plain:?}");
        assert!(
            rows[1].starts_with("       child output stored"),
            "{plain:?}"
        );
        for row in rows {
            assert_eq!(visible_len(row), 48);
        }
    }

    #[test]
    fn compacts_to_tail_with_omitted_count() {
        let rendered = ConnectorBlock::text("one\ntwo\nthree\nfour")
            .max_rows(2)
            .view(40);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("… +2 earlier lines"));
        assert!(!plain.contains("one"));
        assert!(plain.contains("three"));
        assert!(plain.contains("four"));
    }

    #[test]
    fn can_repeat_connector_for_live_output_rows() {
        let rendered = ConnectorBlock::text("one\ntwo\nthree\nfour")
            .connector("│")
            .connector_gap(1)
            .repeat_connector(true)
            .max_rows(2)
            .view(32);
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert_eq!(rows.len(), 3);
        assert!(rows[0].starts_with("    │ … +2 earlier lines"), "{plain}");
        assert!(rows[1].starts_with("    │ three"), "{plain}");
        assert!(rows[2].starts_with("    │ four"), "{plain}");
        for row in rows {
            assert_eq!(visible_len(row), 32);
        }
    }

    #[test]
    fn per_row_color_overrides_default_text_color() {
        let rendered = ConnectorBlock::new()
            .text_color(Color::BrightBlack)
            .row(ConnectorRow::new("failed").color(Color::Red))
            .view(32);

        assert!(rendered.contains("\x1b[31mfailed\x1b[0m"));
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let block = ConnectorBlock::new().with_theme(&theme);

        assert_eq!(block.text_color, theme.color(ThemeRole::Muted));
        assert_eq!(block.omitted_color, theme.color(ThemeRole::Muted));
        assert_eq!(block.connector_color, theme.color(ThemeRole::Border));
    }

    #[test]
    fn cjk_rows_fit_requested_width() {
        let rendered = ConnectorBlock::new()
            .line("中文输出内容 with long suffix")
            .line("下一行内容")
            .view(24);

        for row in rendered.lines() {
            assert_eq!(visible_len(row), 24, "{row:?}");
        }
    }

    #[test]
    fn oversized_spacing_is_clamped_to_render_width() {
        let block = ConnectorBlock::new()
            .margin(usize::MAX)
            .connector_indent(usize::MAX)
            .connector_gap(usize::MAX)
            .line("one")
            .line("two");
        let rendered = block.view(8);

        assert_eq!(block.margin, MAX_CONNECTOR_BLOCK_MARGIN);
        assert_eq!(block.connector_indent, MAX_CONNECTOR_BLOCK_INDENT);
        assert_eq!(block.connector_gap, MAX_CONNECTOR_BLOCK_GAP);
        assert!(rendered.lines().all(|line| visible_len(line) == 8));

        let Element::Box(column) = block.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Box(first_row) = &column.children[0] else {
            panic!("expected first row");
        };
        let Element::Text(margin) = &first_row.children[0] else {
            panic!("expected margin text");
        };
        let Element::Text(gap) = &first_row.children[2] else {
            panic!("expected gap text");
        };
        assert_eq!(
            margin.content.len(),
            MAX_CONNECTOR_BLOCK_MARGIN + MAX_CONNECTOR_BLOCK_INDENT
        );
        assert_eq!(gap.content.len(), MAX_CONNECTOR_BLOCK_GAP);

        let Element::Box(second_row) = &column.children[1] else {
            panic!("expected second row");
        };
        let Element::Text(continuation) = &second_row.children[0] else {
            panic!("expected continuation prefix");
        };
        assert_eq!(
            continuation.content.len(),
            MAX_CONNECTOR_BLOCK_MARGIN
                + MAX_CONNECTOR_BLOCK_INDENT
                + visible_len(&block.connector)
                + MAX_CONNECTOR_BLOCK_GAP
        );
    }

    #[test]
    fn oversized_row_limit_is_clamped() {
        let block = ConnectorBlock::text("one\ntwo").max_rows(usize::MAX);
        let rendered = block.view(8);

        assert_eq!(block.max_rows, Some(MAX_CONNECTOR_BLOCK_ROWS));
        assert_eq!(block.visible_rows().len(), 2);
        assert!(rendered.lines().all(|line| visible_len(line) == 8));
    }

    #[test]
    fn element_produces_structured_rows() {
        let element: Element<()> = ConnectorBlock::new()
            .connector_color(Color::Cyan)
            .line("one")
            .line("two")
            .element();

        let Element::Box(column) = element else {
            panic!("expected column");
        };
        assert_eq!(column.children.len(), 2);

        let Element::Box(first_row) = &column.children[0] else {
            panic!("expected row");
        };
        let Element::Text(connector) = &first_row.children[1] else {
            panic!("expected connector text");
        };
        assert_eq!(connector.content, "⎿");
        assert_eq!(connector.style.fg, Some(Color::Cyan));
    }
}
