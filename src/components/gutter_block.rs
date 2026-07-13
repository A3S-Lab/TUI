use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{
    fit_visible, split_lines_preserving_trailing_blank, strip_ansi, visible_len, Color, Style,
};
use crate::theme::{Theme, ThemeRole};

const MAX_GUTTER_BLOCK_MARGIN: usize = u16::MAX as usize;
const MAX_GUTTER_BLOCK_WIDTH: usize = u16::MAX as usize;

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
        let mut lines = split_lines_preserving_trailing_blank(content.as_ref())
            .into_iter()
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
        self.margin = margin.min(MAX_GUTTER_BLOCK_MARGIN);
        self
    }

    pub fn gap(mut self, gap: impl Into<String>) -> Self {
        self.gap = gap.into();
        self
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width.min(MAX_GUTTER_BLOCK_WIDTH));
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

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.marker_color = theme.color(ThemeRole::Primary);
        self.content_color = Some(theme.color(ThemeRole::Foreground));
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
        let margin_width = self.margin_for_render();
        let margin = " ".repeat(margin_width);
        let inner = self.inner_plain(index, line);

        if let Some(background) = self.background_color {
            let inner_width = self
                .width
                .map(|width| width.saturating_sub(margin_width))
                .unwrap_or_else(|| visible_len(&inner));
            let prefix = if index == 0 {
                format!("{}{}", self.marker, self.gap)
            } else {
                " ".repeat(visible_len(&self.marker) + visible_len(&self.gap))
            };
            let prefix_width = visible_len(&prefix);
            if prefix_width >= inner_width {
                let fitted = fit_visible(&inner, inner_width);
                return format!("{margin}{}", Style::new().bg(background).render(&fitted));
            }

            let content_width = inner_width.saturating_sub(prefix_width);
            let content = fit_visible(&strip_ansi(line), content_width);
            let prefix = if index == 0 {
                format!(
                    "{}{}",
                    self.marker_style().bg(background).render(&self.marker),
                    Style::new().bg(background).render(&self.gap)
                )
            } else {
                Style::new().bg(background).render(&prefix)
            };
            let mut content_style = Style::new().bg(background);
            if let Some(color) = self.content_color {
                content_style = content_style.fg(color);
            }
            return format!("{margin}{prefix}{}", content_style.render(&content));
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
        let mut children = vec![Element::Text(TextElement::new(
            " ".repeat(self.margin_for_render()),
        ))];
        let background = self.background_color;
        if index == 0 {
            let mut marker = TextElement::new(self.marker.clone()).fg(self.marker_color);
            if self.marker_bold {
                marker = marker.bold();
            }
            if let Some(background) = background {
                marker = marker.bg(background);
            }
            children.push(Element::Text(marker));
            let mut gap = TextElement::new(self.gap.clone());
            if let Some(background) = background {
                gap = gap.bg(background);
            }
            children.push(Element::Text(gap));
        } else {
            let mut prefix =
                TextElement::new(" ".repeat(visible_len(&self.marker) + visible_len(&self.gap)));
            if let Some(background) = background {
                prefix = prefix.bg(background);
            }
            children.push(Element::Text(prefix));
        }

        let prefix_width = visible_len(&self.marker) + visible_len(&self.gap);
        let content_width = self
            .width
            .map(|width| {
                width
                    .saturating_sub(self.margin_for_render())
                    .saturating_sub(prefix_width)
            })
            .unwrap_or_else(|| visible_len(&strip_ansi(line)));
        let plain = if background.is_some() {
            fit_visible(&strip_ansi(line), content_width)
        } else {
            strip_ansi(line)
        };
        let mut content = TextElement::new(plain);
        if let Some(color) = self.content_color {
            content = content.fg(color);
        }
        if let Some(background) = background {
            content = content.bg(background);
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

    fn margin_for_render(&self) -> usize {
        self.width
            .map(|width| self.margin.min(width))
            .unwrap_or(self.margin)
            .min(MAX_GUTTER_BLOCK_MARGIN)
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
    fn background_keeps_marker_and_content_styles_separate_across_full_width() {
        let block = GutterBlock::new("hello\nworld")
            .margin(2)
            .width(12)
            .marker_color(Color::Green)
            .content_color(Color::White)
            .background_color(Color::Rgb(38, 45, 64));
        let rendered = block.view();
        let rows = rendered.lines().collect::<Vec<_>>();

        assert_eq!(rows.len(), 2);
        assert!(rendered.contains("\x1b[1;32;48;2;38;45;64m●\x1b[0m"));
        assert!(rendered.contains("\x1b[37;48;2;38;45;64mhello   \x1b[0m"));
        assert_eq!(visible_len(rows[0]), 12);
        assert_eq!(strip_ansi(rows[0]), "  ● hello   ");
        assert_eq!(strip_ansi(rows[1]), "    world   ");

        let Element::Box(column) = block.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Box(first_row) = &column.children[0] else {
            panic!("expected first row");
        };
        let Element::Text(marker) = &first_row.children[1] else {
            panic!("expected marker text");
        };
        let Element::Text(content) = &first_row.children[3] else {
            panic!("expected content text");
        };
        assert_eq!(marker.style.fg, Some(Color::Green));
        assert_eq!(marker.style.bg, Some(Color::Rgb(38, 45, 64)));
        assert!(marker.style.bold);
        assert_eq!(content.style.fg, Some(Color::White));
        assert_eq!(content.style.bg, Some(Color::Rgb(38, 45, 64)));
        assert_eq!(content.content, "hello   ");
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
    fn oversized_margin_is_clamped_to_render_width() {
        let block = GutterBlock::new("hello\nworld").margin(usize::MAX).width(8);
        let rendered = block.view();
        let rows = rendered.lines().collect::<Vec<_>>();

        assert_eq!(block.margin, MAX_GUTTER_BLOCK_MARGIN);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| visible_len(row) == 8));

        let Element::Box(column) = block.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Box(row) = &column.children[0] else {
            panic!("expected row element");
        };
        let Element::Text(margin) = &row.children[0] else {
            panic!("expected margin text");
        };
        assert_eq!(margin.content.len(), 8);
    }

    #[test]
    fn oversized_width_is_clamped() {
        let block = GutterBlock::new("hello").width(usize::MAX);
        let rendered = block.view();

        assert_eq!(block.width, Some(MAX_GUTTER_BLOCK_WIDTH));
        assert_eq!(visible_len(&rendered), MAX_GUTTER_BLOCK_WIDTH);
    }

    #[test]
    fn lines_constructor_keeps_one_empty_line_for_empty_input() {
        let rendered = GutterBlock::lines(Vec::<String>::new()).view();

        assert_eq!(strip_ansi(&rendered), "  ● ");
    }

    #[test]
    fn new_preserves_trailing_blank_row() {
        let block = GutterBlock::new("hello\n");
        let rendered = block.view();
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert_eq!(rows, vec!["  ● hello", "    "]);

        let Element::Box(column) = block.element::<()>() else {
            panic!("expected column element");
        };
        assert_eq!(column.children.len(), 2);
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

    #[test]
    fn with_theme_applies_semantic_colors_without_setting_background() {
        let theme = Theme::tokyo_night();
        let block = GutterBlock::new("hello").with_theme(&theme);

        assert_eq!(block.marker_color, theme.color(ThemeRole::Primary));
        assert_eq!(
            block.content_color,
            Some(theme.color(ThemeRole::Foreground))
        );
        assert_eq!(block.background_color, None);
    }
}
