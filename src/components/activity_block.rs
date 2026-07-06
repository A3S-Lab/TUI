use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{
    fit_visible, split_nonempty_lines_preserving_trailing_blank, truncate_visible, visible_len,
    Color, Style,
};
use crate::theme::{Theme, ThemeRole};

const MAX_ACTIVITY_BLOCK_MARGIN: usize = u16::MAX as usize;
const MAX_ACTIVITY_BLOCK_OUTPUT_LINES: usize = u16::MAX as usize;
const MAX_ACTIVITY_BLOCK_OUTPUT_MARGIN: usize = u16::MAX as usize;
const MAX_ACTIVITY_BLOCK_WIDTH: usize = u16::MAX as usize;

/// In-flight activity line with an optional live output tail.
///
/// This covers CLI feedback such as a running tool: a margin, an active/inactive
/// marker, an action label with detail, and recent output rows prefixed by a
/// muted connector.
#[derive(Debug, Clone)]
pub struct ActivityBlock {
    label: String,
    detail: Option<String>,
    detail_styled: bool,
    lines: Vec<String>,
    max_output_lines: usize,
    margin: usize,
    width: Option<usize>,
    marker: String,
    connector: String,
    active: bool,
    show_ellipsis: bool,
    label_color: Color,
    detail_color: Color,
    active_color: Color,
    inactive_color: Color,
    output_color: Color,
    connector_color: Color,
}

impl ActivityBlock {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: None,
            detail_styled: false,
            lines: Vec::new(),
            max_output_lines: 12,
            margin: 2,
            width: None,
            marker: "•".to_string(),
            connector: "│".to_string(),
            active: true,
            show_ellipsis: true,
            label_color: Color::BrightWhite,
            detail_color: Color::BrightBlack,
            active_color: Color::Yellow,
            inactive_color: Color::BrightBlack,
            output_color: Color::BrightBlack,
            connector_color: Color::BrightBlack,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        let detail = detail.into();
        if !detail.is_empty() {
            self.detail = Some(detail);
            self.detail_styled = false;
        }
        self
    }

    pub fn styled_detail(mut self, detail: impl Into<String>) -> Self {
        let detail = detail.into();
        if !detail.is_empty() {
            self.detail = Some(detail);
            self.detail_styled = true;
        }
        self
    }

    pub fn lines(mut self, lines: Vec<impl Into<String>>) -> Self {
        self.lines = lines.into_iter().map(Into::into).collect();
        self
    }

    pub fn text(mut self, text: impl AsRef<str>) -> Self {
        self.lines = split_nonempty_lines_preserving_trailing_blank(text.as_ref())
            .into_iter()
            .map(str::to_string)
            .collect();
        self
    }

    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.lines.push(line.into());
        self
    }

    pub fn add_line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    pub fn max_output_lines(mut self, max_output_lines: usize) -> Self {
        self.max_output_lines = max_output_lines.clamp(1, MAX_ACTIVITY_BLOCK_OUTPUT_LINES);
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin.min(MAX_ACTIVITY_BLOCK_MARGIN);
        self
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width.min(MAX_ACTIVITY_BLOCK_WIDTH));
        self
    }

    pub fn marker(mut self, marker: impl Into<String>) -> Self {
        self.marker = marker.into();
        self
    }

    pub fn connector(mut self, connector: impl Into<String>) -> Self {
        self.connector = connector.into();
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn show_ellipsis(mut self, show: bool) -> Self {
        self.show_ellipsis = show;
        self
    }

    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
        self
    }

    pub fn detail_color(mut self, color: Color) -> Self {
        self.detail_color = color;
        self
    }

    pub fn marker_colors(mut self, active: Color, inactive: Color) -> Self {
        self.active_color = active;
        self.inactive_color = inactive;
        self
    }

    pub fn output_color(mut self, color: Color) -> Self {
        self.output_color = color;
        self
    }

    pub fn connector_color(mut self, color: Color) -> Self {
        self.connector_color = color;
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.label_color = theme.color(ThemeRole::Primary);
        self.detail_color = theme.color(ThemeRole::Muted);
        self.active_color = theme.color(ThemeRole::Primary);
        self.inactive_color = theme.color(ThemeRole::Muted);
        self.output_color = theme.color(ThemeRole::Muted);
        self.connector_color = theme.color(ThemeRole::Border);
        self
    }

    pub fn label_value(&self) -> &str {
        &self.label
    }

    pub fn detail_value(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    pub fn lines_value(&self) -> &[String] {
        &self.lines
    }

    pub fn view(&self) -> String {
        self.render_lines()
            .into_iter()
            .map(|line| {
                self.width
                    .map(|width| fit_visible(&line, width))
                    .unwrap_or(line)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children = Vec::with_capacity(1 + self.output_tail().len());
        children.push(self.header_element());
        children.extend(
            self.output_tail()
                .into_iter()
                .map(|line| self.output_element(line)),
        );

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_lines(&self) -> Vec<String> {
        let mut lines = vec![self.render_header()];
        lines.extend(
            self.output_tail()
                .into_iter()
                .map(|line| self.render_output(line)),
        );
        lines
    }

    fn render_header(&self) -> String {
        let margin = self.margin_for_render();
        let mut out =
            String::with_capacity(margin.saturating_add(self.label.len()).saturating_add(8));
        out.push_str(&" ".repeat(margin));
        out.push_str(
            &Style::new()
                .fg(self.marker_color())
                .bold()
                .render(&self.marker),
        );
        out.push(' ');
        out.push_str(&Style::new().fg(self.label_color).bold().render(&self.label));

        if let Some(detail) = self.detail.as_deref() {
            out.push(' ');
            let detail_width = self.detail_width(&out);
            let detail = truncate_visible(detail, detail_width);
            if self.detail_styled {
                out.push_str(&detail);
            } else {
                out.push_str(&Style::new().fg(self.detail_color).render(&detail));
            }
        }
        if self.show_ellipsis {
            out.push('…');
        }
        out
    }

    fn render_output(&self, line: String) -> String {
        format!(
            "{}{} {}",
            " ".repeat(self.output_margin_for_render()),
            Style::new()
                .fg(self.connector_color)
                .render(&self.connector),
            Style::new().fg(self.output_color).render(&line)
        )
    }

    fn header_element<Msg>(&self) -> Element<Msg> {
        let mut children = vec![
            Element::Text(TextElement::new(" ".repeat(self.margin_for_render()))),
            Element::Text(
                TextElement::new(self.marker.as_str())
                    .fg(self.marker_color())
                    .bold(),
            ),
            Element::Text(TextElement::new(" ")),
            Element::Text(
                TextElement::new(self.label.as_str())
                    .fg(self.label_color)
                    .bold(),
            ),
        ];

        if let Some(detail) = self.detail.as_deref() {
            children.push(Element::Text(TextElement::new(" ")));
            let text = if self.detail_styled {
                TextElement::new(detail)
            } else {
                TextElement::new(detail).fg(self.detail_color)
            };
            children.push(Element::Text(text));
        }
        if self.show_ellipsis {
            children.push(Element::Text(TextElement::new("…")));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }

    fn output_element<Msg>(&self, line: String) -> Element<Msg> {
        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .child(Element::Text(TextElement::new(
                    " ".repeat(self.output_margin_for_render()),
                )))
                .child(Element::Text(
                    TextElement::new(self.connector.as_str()).fg(self.connector_color),
                ))
                .child(Element::Text(TextElement::new(" ")))
                .child(Element::Text(TextElement::new(line).fg(self.output_color))),
        )
    }

    fn output_tail(&self) -> Vec<String> {
        let start = self.lines.len().saturating_sub(self.max_output_lines);
        self.lines
            .iter()
            .skip(start)
            .map(|line| line.trim_end_matches('\r').to_string())
            .collect()
    }

    fn detail_width(&self, rendered_prefix: &str) -> usize {
        self.width
            .map(|width| {
                let ellipsis = usize::from(self.show_ellipsis);
                width
                    .saturating_sub(visible_len(rendered_prefix) + ellipsis)
                    .max(1)
            })
            .unwrap_or(usize::MAX / 4)
    }

    fn margin_for_render(&self) -> usize {
        self.width
            .map(|width| self.margin.min(width))
            .unwrap_or(self.margin)
            .min(MAX_ACTIVITY_BLOCK_MARGIN)
    }

    fn output_margin_for_render(&self) -> usize {
        let margin = self.margin_for_render();
        let output_margin = margin
            .saturating_add(visible_len(&self.marker))
            .saturating_add(1)
            .min(MAX_ACTIVITY_BLOCK_OUTPUT_MARGIN);
        self.width
            .map(|width| output_margin.min(width))
            .unwrap_or(output_margin)
    }

    fn marker_color(&self) -> Color {
        if self.active {
            self.active_color
        } else {
            self.inactive_color
        }
    }
}

impl Default for ActivityBlock {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn renders_active_header_and_output_tail() {
        let rendered = ActivityBlock::new("Running")
            .detail("cargo test")
            .text("one\ntwo\nthree")
            .max_output_lines(2)
            .width(32)
            .view();
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert_eq!(rows.len(), 3);
        assert!(rows[0].starts_with("  • Running cargo test…"));
        assert!(rows[1].starts_with("    │ two"));
        assert!(rows[2].starts_with("    │ three"));
        assert!(rendered.lines().all(|line| visible_len(line) == 32));
        assert!(rendered.contains("\x1b[1;33m•\x1b[0m"));
    }

    #[test]
    fn text_preserves_trailing_blank_output_line() {
        let block = ActivityBlock::new("Running")
            .show_ellipsis(false)
            .text("done\n");

        assert_eq!(block.lines_value(), ["done", ""]);

        let plain = strip_ansi(&block.view());
        let rows = plain.split('\n').collect::<Vec<_>>();
        assert_eq!(rows[1], "    │ done");
        assert_eq!(rows[2], "    │ ");
    }

    #[test]
    fn empty_text_keeps_output_empty() {
        let block = ActivityBlock::new("Running").text("");

        assert!(block.lines_value().is_empty());
    }

    #[test]
    fn inactive_marker_uses_inactive_color() {
        let rendered = ActivityBlock::new("Waiting")
            .active(false)
            .show_ellipsis(false)
            .view();

        assert_eq!(strip_ansi(&rendered), "  • Waiting");
        assert!(rendered.contains("\x1b[1;90m•\x1b[0m"));
    }

    #[test]
    fn truncates_detail_to_configured_width() {
        let rendered = ActivityBlock::new("Running")
            .detail("very long command with 中文 payload")
            .width(20)
            .view();

        assert!(rendered.lines().all(|line| visible_len(line) == 20));
        assert!(strip_ansi(&rendered).contains('…'));
    }

    #[test]
    fn styled_detail_preserves_ansi_and_fits_width() {
        let detail = Style::new()
            .fg(Color::Cyan)
            .render("cargo test -- --nocapture");
        let rendered = ActivityBlock::new("Running")
            .styled_detail(detail)
            .width(24)
            .view();

        assert!(rendered.contains("\x1b[36mcargo"));
        assert_eq!(visible_len(rendered.lines().next().unwrap()), 24);
        assert!(strip_ansi(&rendered).starts_with("  • Running cargo"));
    }

    #[test]
    fn plain_detail_uses_detail_color() {
        let rendered = ActivityBlock::new("Running").detail("plain").view();

        assert!(rendered.contains("\x1b[90mplain\x1b[0m"));
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let block = ActivityBlock::new("Running").with_theme(&theme);

        assert_eq!(block.label_color, theme.color(ThemeRole::Primary));
        assert_eq!(block.detail_color, theme.color(ThemeRole::Muted));
        assert_eq!(block.active_color, theme.color(ThemeRole::Primary));
        assert_eq!(block.inactive_color, theme.color(ThemeRole::Muted));
        assert_eq!(block.output_color, theme.color(ThemeRole::Muted));
        assert_eq!(block.connector_color, theme.color(ThemeRole::Border));
    }

    #[test]
    fn custom_margin_marker_and_connector_align_output() {
        let rendered = ActivityBlock::new("Sync")
            .marker(">>")
            .connector("|")
            .margin(1)
            .line("done")
            .view();
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert!(rows[0].starts_with(" >> Sync…"));
        assert_eq!(rows[1], "    | done");
    }

    #[test]
    fn oversized_margin_is_clamped_to_render_width() {
        let block = ActivityBlock::new("Running")
            .margin(usize::MAX)
            .width(8)
            .line("ok");
        let rendered = block.view();
        let rows = rendered.lines().collect::<Vec<_>>();

        assert_eq!(block.margin, MAX_ACTIVITY_BLOCK_MARGIN);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| visible_len(row) == 8));

        let Element::Box(column) = block.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Box(header) = &column.children[0] else {
            panic!("expected header row");
        };
        let Element::Text(margin) = &header.children[0] else {
            panic!("expected margin text");
        };
        assert_eq!(margin.content.len(), 8);

        let Element::Box(output) = &column.children[1] else {
            panic!("expected output row");
        };
        let Element::Text(indent) = &output.children[0] else {
            panic!("expected output indent text");
        };
        assert_eq!(indent.content.len(), 8);
    }

    #[test]
    fn oversized_width_is_clamped() {
        let block = ActivityBlock::new("Running")
            .detail("cargo test")
            .width(usize::MAX)
            .line("ok");
        let rendered = block.view();

        assert_eq!(block.width, Some(MAX_ACTIVITY_BLOCK_WIDTH));
        assert!(rendered
            .lines()
            .all(|line| visible_len(line) == MAX_ACTIVITY_BLOCK_WIDTH));
    }

    #[test]
    fn oversized_output_line_limit_is_clamped() {
        let block = ActivityBlock::new("Running")
            .max_output_lines(usize::MAX)
            .width(24)
            .text("one\ntwo");
        let rendered = block.view();

        assert_eq!(block.max_output_lines, MAX_ACTIVITY_BLOCK_OUTPUT_LINES);
        assert_eq!(block.output_tail().len(), 2);
        assert!(rendered.lines().all(|line| visible_len(line) == 24));
    }

    #[test]
    fn element_produces_structured_rows() {
        let element: Element<()> = ActivityBlock::new("Running")
            .detail("cargo test")
            .line("ok")
            .marker_colors(Color::Cyan, Color::BrightBlack)
            .element();

        match element {
            Element::Box(column) => {
                assert_eq!(column.children.len(), 2);
                match &column.children[0] {
                    Element::Box(row) => match &row.children[1] {
                        Element::Text(marker) => {
                            assert_eq!(marker.content, "•");
                            assert_eq!(marker.style.fg, Some(Color::Cyan));
                            assert!(marker.style.bold);
                        }
                        _ => panic!("expected marker text"),
                    },
                    _ => panic!("expected header row"),
                }
            }
            _ => panic!("expected column element"),
        }
    }
}
