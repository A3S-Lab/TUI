use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};

/// Visual role for a row inside a [`DetailPanel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailRowKind {
    #[default]
    Text,
    KeyValue,
    Action,
    Muted,
}

/// One row in a [`DetailPanel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailRow {
    label: Option<String>,
    value: String,
    kind: DetailRowKind,
    color: Option<Color>,
    bold: bool,
}

impl DetailRow {
    pub fn text(value: impl Into<String>) -> Self {
        Self {
            label: None,
            value: value.into(),
            kind: DetailRowKind::Text,
            color: None,
            bold: false,
        }
    }

    pub fn pair(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            value: value.into(),
            kind: DetailRowKind::KeyValue,
            color: None,
            bold: false,
        }
    }

    pub fn action(value: impl Into<String>) -> Self {
        Self {
            label: Some("actions".to_string()),
            value: value.into(),
            kind: DetailRowKind::Action,
            color: None,
            bold: false,
        }
    }

    pub fn muted(value: impl Into<String>) -> Self {
        Self {
            label: None,
            value: value.into(),
            kind: DetailRowKind::Muted,
            color: None,
            bold: false,
        }
    }

    pub fn kind(mut self, kind: DetailRowKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn kind_value(&self) -> DetailRowKind {
        self.kind
    }
}

/// Compact details surface for selected rows, resources, events, and actions.
///
/// This component extracts the common CLI pattern of a separator, bold heading,
/// key/value metadata, and an action hint row. It keeps every rendered line at
/// the requested display width, including styled text and CJK content.
#[derive(Debug, Clone)]
pub struct DetailPanel {
    title: Option<String>,
    rows: Vec<DetailRow>,
    max_rows: Option<usize>,
    show_separator: bool,
    fill_height: bool,
    indent: usize,
    label_width: Option<usize>,
    separator_color: Color,
    title_color: Color,
    label_color: Color,
    value_color: Color,
    action_color: Color,
    muted_color: Color,
}

impl DetailPanel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            rows: Vec::new(),
            max_rows: Some(10),
            show_separator: true,
            fill_height: false,
            indent: 1,
            label_width: None,
            separator_color: Color::BrightBlack,
            title_color: Color::Cyan,
            label_color: Color::BrightBlack,
            value_color: Color::White,
            action_color: Color::Cyan,
            muted_color: Color::BrightBlack,
        }
    }

    pub fn without_title() -> Self {
        Self {
            title: None,
            ..Self::new("")
        }
    }

    pub fn row(mut self, row: DetailRow) -> Self {
        self.rows.push(row);
        self
    }

    pub fn text(mut self, value: impl Into<String>) -> Self {
        self.rows.push(DetailRow::text(value));
        self
    }

    pub fn pair(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.rows.push(DetailRow::pair(label, value));
        self
    }

    pub fn action(mut self, value: impl Into<String>) -> Self {
        self.rows.push(DetailRow::action(value));
        self
    }

    pub fn add_row(&mut self, row: DetailRow) {
        self.rows.push(row);
    }

    pub fn max_rows(mut self, max_rows: usize) -> Self {
        self.max_rows = Some(max_rows.max(1));
        self
    }

    pub fn unlimited_rows(mut self) -> Self {
        self.max_rows = None;
        self
    }

    pub fn show_separator(mut self, enabled: bool) -> Self {
        self.show_separator = enabled;
        self
    }

    pub fn fill_height(mut self, enabled: bool) -> Self {
        self.fill_height = enabled;
        self
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn label_width(mut self, width: usize) -> Self {
        self.label_width = Some(width);
        self
    }

    pub fn separator_color(mut self, color: Color) -> Self {
        self.separator_color = color;
        self
    }

    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
        self
    }

    pub fn value_color(mut self, color: Color) -> Self {
        self.value_color = color;
        self
    }

    pub fn action_color(mut self, color: Color) -> Self {
        self.action_color = color;
        self
    }

    pub fn muted_color(mut self, color: Color) -> Self {
        self.muted_color = color;
        self
    }

    pub fn rows(&self) -> &[DetailRow] {
        &self.rows
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        let mut lines = self.render_lines(width, height);
        lines.truncate(height);
        if self.fill_height {
            while lines.len() < height {
                lines.push(String::new());
            }
        }

        lines
            .into_iter()
            .map(|line| fit_visible(&line, width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children = Vec::new();
        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            children.push(Element::Text(
                TextElement::new(title).fg(self.title_color).bold(),
            ));
        }
        for row in self.rows.iter().take(self.max_rows.unwrap_or(usize::MAX)) {
            children.push(self.row_element(row));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_lines(&self, width: usize, height: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if self.show_separator && height > 0 {
            lines.push(
                Style::new()
                    .fg(self.separator_color)
                    .render(&"─".repeat(width)),
            );
        }
        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            if lines.len() < height {
                lines.push(
                    Style::new()
                        .fg(self.title_color)
                        .bold()
                        .render(&fit_visible(
                            &format!("{}{}", " ".repeat(self.indent), title),
                            width,
                        )),
                );
            }
        }

        let available = height.saturating_sub(lines.len());
        let limit = self
            .max_rows
            .unwrap_or(usize::MAX)
            .min(available)
            .min(self.rows.len());
        for row in self.rows.iter().take(limit) {
            lines.push(self.render_row(row, width));
        }
        lines
    }

    fn render_row(&self, row: &DetailRow, width: usize) -> String {
        let indent = " ".repeat(self.indent);
        let available = width.saturating_sub(self.indent);
        match row.kind {
            DetailRowKind::KeyValue | DetailRowKind::Action => {
                let label = row.label.as_deref().unwrap_or_default();
                let label_width = self
                    .label_width
                    .unwrap_or_else(|| self.computed_label_width())
                    .min(available);
                let label_text = fit_visible(label, label_width);
                let gap = " ";
                let value_width = available.saturating_sub(label_width + visible_len(gap));
                let value = truncate_visible(&row.value, value_width);
                format!(
                    "{indent}{}{}{}",
                    Style::new()
                        .fg(self.row_label_color(row))
                        .bold()
                        .render(&label_text),
                    gap,
                    self.row_value_style(row).render(&value)
                )
            }
            DetailRowKind::Text | DetailRowKind::Muted => {
                let value = truncate_visible(&row.value, available);
                format!("{indent}{}", self.row_value_style(row).render(&value))
            }
        }
    }

    fn row_element<Msg>(&self, row: &DetailRow) -> Element<Msg> {
        match row.kind {
            DetailRowKind::KeyValue | DetailRowKind::Action => Element::Box(
                BoxElement::new()
                    .direction(FlexDirection::Row)
                    .child(Element::Text(
                        TextElement::new(row.label.as_deref().unwrap_or_default())
                            .fg(self.row_label_color(row))
                            .bold(),
                    ))
                    .child(Element::Text(TextElement::new(" ")))
                    .child(Element::Text(self.text_element(
                        &row.value,
                        self.row_value_color(row),
                        row.bold,
                    ))),
            ),
            DetailRowKind::Text | DetailRowKind::Muted => {
                Element::Text(self.text_element(&row.value, self.row_value_color(row), row.bold))
            }
        }
    }

    fn text_element(&self, text: &str, color: Color, bold: bool) -> TextElement {
        let mut element = TextElement::new(text).fg(color);
        if bold {
            element = element.bold();
        }
        element
    }

    fn row_value_style(&self, row: &DetailRow) -> Style {
        let mut style = Style::new().fg(self.row_value_color(row));
        if row.bold {
            style = style.bold();
        }
        style
    }

    fn row_label_color(&self, row: &DetailRow) -> Color {
        if row.kind == DetailRowKind::Action {
            self.action_color
        } else {
            self.label_color
        }
    }

    fn row_value_color(&self, row: &DetailRow) -> Color {
        row.color.unwrap_or(match row.kind {
            DetailRowKind::Action => self.action_color,
            DetailRowKind::Muted => self.muted_color,
            DetailRowKind::Text | DetailRowKind::KeyValue => self.value_color,
        })
    }

    fn computed_label_width(&self) -> usize {
        self.rows
            .iter()
            .filter_map(|row| row.label.as_deref())
            .map(visible_len)
            .max()
            .unwrap_or(0)
            .max(1)
    }
}

impl Default for DetailPanel {
    fn default() -> Self {
        Self::without_title()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    fn sample() -> DetailPanel {
        DetailPanel::new("process 42 · risk high")
            .pair("cpu", "91.4%")
            .pair("mem", "512 MiB")
            .text("cwd /Users/roylin/code/a3s")
            .action("o focus · / filter · K terminate")
    }

    #[test]
    fn renders_separator_title_rows_and_actions() {
        let rendered = sample().view(48, 8);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("process 42"));
        assert!(plain.contains("cpu"));
        assert!(plain.contains("91.4%"));
        assert!(plain.contains("actions"));
        assert!(plain.contains("K terminate"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 48, "{line:?}");
        }
    }

    #[test]
    fn respects_height_and_max_rows() {
        let rendered = sample().max_rows(2).view(40, 4);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("cpu"));
        assert!(plain.contains("mem"));
        assert!(!plain.contains("cwd"));
        assert_eq!(rendered.lines().count(), 4);
    }

    #[test]
    fn fill_height_pads_remaining_rows() {
        let rendered = DetailPanel::new("container")
            .pair("image", "postgres")
            .fill_height(true)
            .view(32, 6);

        assert_eq!(rendered.lines().count(), 6);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 32, "{line:?}");
        }
    }

    #[test]
    fn truncates_cjk_values_by_display_width() {
        let rendered = DetailPanel::new("event")
            .pair("message", "中文测试内容 with a long explanatory suffix")
            .view(24, 4);

        for line in rendered.lines() {
            assert_eq!(visible_len(line), 24, "{line:?}");
        }
        assert!(strip_ansi(&rendered).contains("中文"));
    }

    #[test]
    fn custom_label_width_aligns_values() {
        let rendered = DetailPanel::new("meta")
            .label_width(8)
            .pair("pid", "42")
            .pair("workspace", "a3s")
            .view(32, 5);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("pid      42"));
        assert!(plain.contains("workspa… a3s"));
    }

    #[test]
    fn zero_size_renders_empty_string() {
        assert_eq!(sample().view(0, 8), "");
        assert_eq!(sample().view(40, 0), "");
    }

    #[test]
    fn element_produces_column() {
        let el: Element<()> = sample().element();

        match el {
            Element::Box(column) => {
                assert_eq!(column.style.flex_direction, FlexDirection::Column);
                assert!(!column.children.is_empty());
            }
            _ => panic!("expected Box"),
        }
    }
}
