use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};

/// In-flight activity line with an optional live output tail.
///
/// This covers CLI feedback such as a running tool: a margin, an active/inactive
/// marker, an action label with detail, and recent output rows prefixed by a
/// muted connector.
#[derive(Debug, Clone)]
pub struct ActivityBlock {
    label: String,
    detail: Option<String>,
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
        }
        self
    }

    pub fn lines(mut self, lines: Vec<impl Into<String>>) -> Self {
        self.lines = lines.into_iter().map(Into::into).collect();
        self
    }

    pub fn text(mut self, text: impl AsRef<str>) -> Self {
        self.lines = text.as_ref().lines().map(str::to_string).collect();
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
        self.max_output_lines = max_output_lines.max(1);
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin;
        self
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width);
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
        let mut out = String::with_capacity(self.margin + self.label.len() + 8);
        out.push_str(&" ".repeat(self.margin));
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
            out.push_str(
                &Style::new()
                    .fg(self.detail_color)
                    .render(&truncate_visible(detail, detail_width)),
            );
        }
        if self.show_ellipsis {
            out.push('…');
        }
        out
    }

    fn render_output(&self, line: String) -> String {
        format!(
            "{}{} {}",
            " ".repeat(self.output_margin()),
            Style::new()
                .fg(self.connector_color)
                .render(&self.connector),
            Style::new().fg(self.output_color).render(&line)
        )
    }

    fn header_element<Msg>(&self) -> Element<Msg> {
        let mut children = vec![
            Element::Text(TextElement::new(" ".repeat(self.margin))),
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
            children.push(Element::Text(
                TextElement::new(detail).fg(self.detail_color),
            ));
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
                    " ".repeat(self.output_margin()),
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

    fn output_margin(&self) -> usize {
        self.margin + visible_len(&self.marker) + 1
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
