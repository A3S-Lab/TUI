use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, visible_len, wrap_words, wrap_words_compact, Color, Style};

/// Wrapped text block with a first-line prefix and aligned continuation prefix.
///
/// This covers transcript/callout text such as live reasoning: a left margin,
/// a visible prefix on the first wrapped row, a matching continuation prefix,
/// and display-column wrapping that keeps CJK and wide glyphs aligned.
#[derive(Debug, Clone)]
pub struct WrappedPrefixBlock {
    text: String,
    first_prefix: String,
    continuation_prefix: String,
    margin: usize,
    width: Option<usize>,
    style: Option<Style>,
    compact_blank_lines: bool,
}

impl WrappedPrefixBlock {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            first_prefix: String::new(),
            continuation_prefix: String::new(),
            margin: 0,
            width: None,
            style: None,
            compact_blank_lines: true,
        }
    }

    pub fn prefixes(mut self, first: impl Into<String>, continuation: impl Into<String>) -> Self {
        self.first_prefix = first.into();
        self.continuation_prefix = continuation.into();
        self
    }

    pub fn first_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.first_prefix = prefix.into();
        self
    }

    pub fn continuation_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.continuation_prefix = prefix.into();
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

    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.style = Some(Style::new().fg(color));
        self
    }

    pub fn preserve_blank_lines(mut self, preserve: bool) -> Self {
        self.compact_blank_lines = !preserve;
        self
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
        Element::Box(
            BoxElement::new().direction(FlexDirection::Column).children(
                self.plain_lines()
                    .into_iter()
                    .enumerate()
                    .map(|(index, line)| self.element_line(index, line))
                    .collect(),
            ),
        )
    }

    fn render_lines(&self) -> Vec<String> {
        self.plain_lines()
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                let margin = " ".repeat(self.margin);
                let prefix = self.prefix_for(index);
                let text = format!("{prefix}{line}");
                match &self.style {
                    Some(style) => format!("{margin}{}", style.render(&text)),
                    None => format!("{margin}{text}"),
                }
            })
            .collect()
    }

    fn plain_lines(&self) -> Vec<String> {
        let width = self.content_width();
        if self.compact_blank_lines {
            wrap_words_compact(self.text.trim(), width)
        } else {
            wrap_words(&self.text, width)
        }
    }

    fn content_width(&self) -> usize {
        let prefix_width =
            visible_len(&self.first_prefix).max(visible_len(&self.continuation_prefix));
        self.width
            .map(|width| width.saturating_sub(self.margin + prefix_width).max(1))
            .unwrap_or(usize::MAX / 4)
    }

    fn prefix_for(&self, index: usize) -> &str {
        if index == 0 {
            &self.first_prefix
        } else {
            &self.continuation_prefix
        }
    }

    fn element_line<Msg>(&self, index: usize, line: String) -> Element<Msg> {
        let margin = " ".repeat(self.margin);
        let prefix = self.prefix_for(index).to_string();
        let text = format!("{prefix}{line}");
        let mut text_el = TextElement::new(text);
        if let Some(style) = &self.style {
            text_el = apply_text_style(text_el, style);
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .child(Element::Text(TextElement::new(margin)))
                .child(Element::Text(text_el)),
        )
    }
}

fn apply_text_style(mut text: TextElement, style: &Style) -> TextElement {
    if let Some(fg) = style.foreground() {
        text = text.fg(fg);
    }
    if let Some(bg) = style.background() {
        text = text.bg(bg);
    }
    if style.is_bold() {
        text = text.bold();
    }
    if style.is_italic() {
        text = text.italic();
    }
    if style.is_underline() {
        text = text.underline();
    }
    if style.is_dim() {
        text = text.dim();
    }
    if style.is_strikethrough() {
        text = text.strikethrough();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn wraps_with_first_and_continuation_prefixes() {
        let rendered = WrappedPrefixBlock::new("alpha beta gamma delta")
            .margin(2)
            .width(14)
            .prefixes("💭 ", "   ")
            .view();
        let rows = strip_ansi(&rendered)
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();

        assert!(rows[0].starts_with("  💭 alpha"));
        assert!(rows[1].starts_with("     beta"));
        assert!(rows[2].starts_with("     gamma"));
        assert!(rows[3].starts_with("     delta"));
        assert!(rendered.lines().all(|line| visible_len(line) == 14));
    }

    #[test]
    fn styles_prefix_and_content_but_not_margin() {
        let rendered = WrappedPrefixBlock::new("thinking")
            .margin(2)
            .prefixes("💭 ", "   ")
            .style(Style::new().fg(Color::BrightBlack).italic())
            .view();

        assert!(rendered.starts_with("  \x1b[3;90m"));
        assert_eq!(strip_ansi(&rendered), "  💭 thinking");
    }

    #[test]
    fn compact_blank_lines_by_default() {
        let rendered = WrappedPrefixBlock::new("one\n\n\n two")
            .prefixes("> ", "  ")
            .view();

        assert_eq!(strip_ansi(&rendered).lines().count(), 2);
    }

    #[test]
    fn can_preserve_blank_lines() {
        let rendered = WrappedPrefixBlock::new("one\n\n two")
            .prefixes("> ", "  ")
            .preserve_blank_lines(true)
            .view();

        assert_eq!(
            strip_ansi(&rendered).lines().collect::<Vec<_>>(),
            vec!["> one", "  ", "  two"]
        );
    }

    #[test]
    fn wraps_cjk_by_display_width() {
        let rendered = WrappedPrefixBlock::new("中文测试内容")
            .prefixes("💭 ", "   ")
            .width(11)
            .view();
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert!(rendered.lines().all(|line| visible_len(line) <= 11));
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("💭 中文测试"));
        assert!(rows[1].starts_with("   内容"));
    }

    #[test]
    fn element_produces_structured_lines() {
        let element: Element<()> = WrappedPrefixBlock::new("alpha beta")
            .prefixes("> ", "  ")
            .width(8)
            .color(Color::Cyan)
            .element();

        match element {
            Element::Box(column) => {
                assert_eq!(column.children.len(), 2);
                match &column.children[0] {
                    Element::Box(row) => {
                        assert_eq!(row.children.len(), 2);
                        match &row.children[1] {
                            Element::Text(text) => {
                                assert!(text.content.starts_with("> "));
                                assert_eq!(text.style.fg, Some(Color::Cyan));
                            }
                            _ => panic!("expected text element"),
                        }
                    }
                    _ => panic!("expected row element"),
                }
            }
            _ => panic!("expected column element"),
        }
    }
}
