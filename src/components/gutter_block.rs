use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, strip_ansi, visible_len, Color, Style};

/// Transcript-style block with a marker on the first row and aligned continuation rows.
///
/// This extracts the common terminal transcript pattern used for chat messages,
/// status notes, and restored history: a colored marker such as `●`, a fixed
/// left margin, and continuation lines aligned under the message text. When a
/// background color and width are set, it also renders full-width input bubbles.
#[derive(Debug, Clone)]
pub struct GutterBlock {
    lines: Vec<String>,
    marker: String,
    margin: usize,
    gap: String,
    width: Option<usize>,
    marker_color: Color,
    marker_bold: bool,
    content_color: Option<Color>,
    background_color: Option<Color>,
}

impl GutterBlock {
    pub fn new(content: impl AsRef<str>) -> Self {
        let mut lines = content
            .as_ref()
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        if lines.is_empty() {
            lines.push(String::new());
        }

        Self {
            lines,
            marker: "●".to_string(),
            margin: 2,
            gap: " ".to_string(),
            width: None,
            marker_color: Color::Cyan,
            marker_bold: true,
            content_color: None,
            background_color: None,
        }
    }

    pub fn lines(lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut block = Self::new("");
        block.lines = lines.into_iter().map(Into::into).collect();
        if block.lines.is_empty() {
            block.lines.push(String::new());
        }
        block
    }

    pub fn marker(mut self, marker: impl Into<String>) -> Self {
        self.marker = marker.into();
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin;
        self
    }

    pub fn gap(mut self, gap: impl Into<String>) -> Self {
        self.gap = gap.into();
        self
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    pub fn marker_color(mut self, color: Color) -> Self {
        self.marker_color = color;
        self
    }

    pub fn marker_bold(mut self, bold: bool) -> Self {
        self.marker_bold = bold;
        self
    }

    pub fn content_color(mut self, color: Color) -> Self {
        self.content_color = Some(color);
        self
    }

    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    pub fn view(&self) -> String {
        self.lines
            .iter()
            .enumerate()
            .map(|(index, line)| self.render_line(index, line))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        Element::Box(
            BoxElement::new().direction(FlexDirection::Column).children(
                self.lines
                    .iter()
                    .enumerate()
                    .map(|(index, line)| self.element_line(index, line))
                    .collect(),
            ),
        )
    }

    fn render_line(&self, index: usize, line: &str) -> String {
        let margin = " ".repeat(self.margin);
        let inner = self.inner_plain(index, line);

        if let Some(background) = self.background_color {
            let inner_width = self
                .width
                .map(|width| width.saturating_sub(self.margin))
                .unwrap_or_else(|| visible_len(&inner));
            let fitted = fit_visible(&inner, inner_width);
            let mut style = Style::new().bg(background);
            if let Some(color) = self.content_color {
                style = style.fg(color);
            }
            return format!("{margin}{}", style.render(&fitted));
        }

        let prefix = if index == 0 {
            format!(
                "{margin}{}{}",
                self.marker_style().render(&self.marker),
                self.gap
            )
        } else {
            format!(
                "{margin}{}",
                " ".repeat(visible_len(&self.marker) + visible_len(&self.gap))
            )
        };
        let content = self.render_content(line);
        let rendered = format!("{prefix}{content}");

        self.width
            .map(|width| fit_visible(&rendered, width))
            .unwrap_or(rendered)
    }

    fn element_line<Msg>(&self, index: usize, line: &str) -> Element<Msg> {
        if self.background_color.is_some() {
            let plain = strip_ansi(&self.render_line(index, line));
            let mut text = TextElement::new(plain);
            if let Some(color) = self.content_color {
                text = text.fg(color);
            }
            if let Some(background) = self.background_color {
                text = text.bg(background);
            }
            return Element::Text(text);
        }

        let mut children = vec![Element::Text(TextElement::new(" ".repeat(self.margin)))];
        if index == 0 {
            let mut marker = TextElement::new(self.marker.clone()).fg(self.marker_color);
            if self.marker_bold {
                marker = marker.bold();
            }
            children.push(Element::Text(marker));
            children.push(Element::Text(TextElement::new(self.gap.clone())));
        } else {
            children.push(Element::Text(TextElement::new(
                " ".repeat(visible_len(&self.marker) + visible_len(&self.gap)),
            )));
        }

        let plain = strip_ansi(line);
        let mut content = TextElement::new(plain);
        if let Some(color) = self.content_color {
            content = content.fg(color);
        }
        children.push(Element::Text(content));

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }

    fn inner_plain(&self, index: usize, line: &str) -> String {
        if index == 0 {
            format!("{}{}{}", self.marker, self.gap, strip_ansi(line))
        } else {
            format!(
                "{}{}",
                " ".repeat(visible_len(&self.marker) + visible_len(&self.gap)),
                strip_ansi(line)
            )
        }
    }

    fn render_content(&self, line: &str) -> String {
        match self.content_color {
            Some(color) => Style::new().fg(color).render(line),
            None => line.to_string(),
        }
    }

    fn marker_style(&self) -> Style {
        let mut style = Style::new().fg(self.marker_color);
        if self.marker_bold {
            style = style.bold();
        }
        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_marker_on_first_line_and_aligns_continuations() {
        let rendered = GutterBlock::new("hello\nworld")
            .margin(2)
            .marker_color(Color::Green)
            .view();
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert_eq!(rows[0], "  ● hello");
        assert_eq!(rows[1], "    world");
        assert!(rendered.contains("\x1b[1;32m●\x1b[0m"));
    }

    #[test]
    fn preserves_styled_content_when_no_content_color_is_set() {
        let styled = Style::new().fg(Color::Yellow).render("styled");
        let rendered = GutterBlock::new(styled).marker_color(Color::Green).view();

        assert!(rendered.contains("\x1b[33mstyled\x1b[0m"));
        assert_eq!(strip_ansi(&rendered), "  ● styled");
    }

    #[test]
    fn content_color_styles_every_content_line() {
        let rendered = GutterBlock::new("one\ntwo")
            .content_color(Color::BrightBlack)
            .view();

        assert!(rendered.contains("\x1b[90mone\x1b[0m"));
        assert!(rendered.contains("\x1b[90mtwo\x1b[0m"));
    }

    #[test]
    fn background_width_renders_user_bubble_shape() {
        let rendered = GutterBlock::new("hello\nworld")
            .margin(2)
            .width(12)
            .content_color(Color::White)
            .background_color(Color::Rgb(38, 45, 64))
            .view();
        let rows = rendered.lines().collect::<Vec<_>>();

        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("  \x1b[37;48;2;38;45;64m"));
        assert_eq!(visible_len(rows[0]), 12);
        assert_eq!(strip_ansi(rows[0]), "  ● hello   ");
        assert_eq!(strip_ansi(rows[1]), "    world   ");
    }

    #[test]
    fn width_truncates_and_pads_regular_block() {
        let rendered = GutterBlock::new("abcdef")
            .marker_color(Color::Green)
            .width(6)
            .view();

        assert_eq!(visible_len(&rendered), 6);
        assert!(strip_ansi(&rendered).ends_with('…'));
    }

    #[test]
    fn lines_constructor_keeps_one_empty_line_for_empty_input() {
        let rendered = GutterBlock::lines(Vec::<String>::new()).view();

        assert_eq!(strip_ansi(&rendered), "  ● ");
    }

    #[test]
    fn element_produces_column_rows() {
        let element: Element<()> = GutterBlock::new("hello\nworld")
            .marker_color(Color::Green)
            .element();

        match element {
            Element::Box(column) => {
                assert_eq!(column.children.len(), 2);
                match &column.children[0] {
                    Element::Box(row) => assert_eq!(row.children.len(), 4),
                    _ => panic!("expected row element"),
                }
            }
            _ => panic!("expected column element"),
        }
    }
}
