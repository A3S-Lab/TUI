use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{center_visible, right_visible, visible_len, wrap_words, Color, Style};

const MAX_PARAGRAPH_INDENT: usize = u16::MAX as usize;
const MAX_PARAGRAPH_WIDTH: usize = u16::MAX as usize;

/// Text alignment for paragraphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// A paragraph component with automatic word wrapping.
///
/// Wraps text to fit within a specified width, with optional indentation
/// and alignment.
pub struct Paragraph {
    text: String,
    width: usize,
    indent: usize,
    align: TextAlign,
    color: Option<Color>,
}

impl Paragraph {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            width: 80,
            indent: 0,
            align: TextAlign::Left,
            color: None,
        }
    }

    pub fn width(mut self, w: usize) -> Self {
        self.width = w.min(MAX_PARAGRAPH_WIDTH);
        self
    }

    pub fn indent(mut self, n: usize) -> Self {
        self.indent = n.min(MAX_PARAGRAPH_INDENT);
        self
    }

    pub fn align(mut self, a: TextAlign) -> Self {
        self.align = a;
        self
    }

    pub fn color(mut self, c: Color) -> Self {
        self.color = Some(c);
        self
    }

    pub fn lines(&self) -> Vec<String> {
        self.wrap()
            .into_iter()
            .map(|line| self.apply_style(&self.apply_align(&line)))
            .collect()
    }

    pub fn view(&self) -> String {
        self.lines().join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let lines = self.wrap();
        let children: Vec<Element<Msg>> = lines
            .into_iter()
            .map(|line| {
                let aligned = self.apply_align(&line);
                let mut el = TextElement::new(aligned);
                if let Some(c) = self.color {
                    el = el.fg(c);
                }
                Element::Text(el)
            })
            .collect();

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn wrap(&self) -> Vec<String> {
        let width = self.width_for_render();
        let indent = self.indent_for_width(width);
        let effective_width = width.saturating_sub(indent);
        if effective_width == 0 {
            return vec![self.text.clone()];
        }

        let indent_str = " ".repeat(indent);
        wrap_words(&self.text, effective_width)
            .into_iter()
            .map(|line| {
                if line.is_empty() {
                    line
                } else {
                    format!("{indent_str}{line}")
                }
            })
            .collect()
    }

    fn apply_align(&self, line: &str) -> String {
        let content = line.trim_start().trim_end();
        let content_len = visible_len(content);
        let available = self.width_for_render();

        match self.align {
            TextAlign::Left => line.to_string(),
            TextAlign::Center => {
                if available == 0 || content_len >= available {
                    content.to_string()
                } else {
                    center_visible(content, available)
                }
            }
            TextAlign::Right => {
                if available == 0 || content_len >= available {
                    content.to_string()
                } else {
                    right_visible(content, available)
                }
            }
        }
    }

    fn apply_style(&self, line: &str) -> String {
        match self.color {
            Some(color) => Style::new().fg(color).render(line),
            None => line.to_string(),
        }
    }

    fn width_for_render(&self) -> usize {
        self.width.min(MAX_PARAGRAPH_WIDTH)
    }

    fn indent_for_width(&self, width: usize) -> usize {
        self.indent.min(width).min(MAX_PARAGRAPH_INDENT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, Style};

    #[test]
    fn no_wrap_short_text() {
        let p = Paragraph::new("hello world").width(80);
        let lines = p.wrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "hello world");
    }

    #[test]
    fn wraps_at_width() {
        let p = Paragraph::new("hello world foo bar").width(10);
        let lines = p.wrap();
        assert!(lines.len() >= 2);
        for line in &lines {
            assert!(visible_len(line) <= 10);
        }
    }

    #[test]
    fn preserves_newlines() {
        let p = Paragraph::new("line1\n\nline2").width(80);
        let lines = p.wrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1], "");
    }

    #[test]
    fn preserves_trailing_newline() {
        let p = Paragraph::new("line1\n").width(80);

        assert_eq!(p.wrap(), vec!["line1", ""]);
    }

    #[test]
    fn indent_adds_spaces() {
        let p = Paragraph::new("hello").width(80).indent(4);
        let lines = p.wrap();
        assert!(lines[0].starts_with("    "));
    }

    #[test]
    fn oversized_dimensions_are_clamped() {
        let p = Paragraph::new("hello").width(usize::MAX).indent(usize::MAX);

        assert_eq!(p.width, MAX_PARAGRAPH_WIDTH);
        assert_eq!(p.indent, MAX_PARAGRAPH_INDENT);
        assert_eq!(p.indent_for_width(8), 8);

        let lines = Paragraph::new("hello").width(8).indent(usize::MAX).wrap();
        assert_eq!(lines, vec!["hello"]);
    }

    #[test]
    fn center_alignment() {
        let p = Paragraph::new("hi").width(10).align(TextAlign::Center);
        let aligned = p.apply_align("hi");
        assert_eq!(visible_len(&aligned), 10);
        assert_eq!(aligned, "    hi    ");
    }

    #[test]
    fn right_alignment() {
        let p = Paragraph::new("hi").width(10).align(TextAlign::Right);
        let aligned = p.apply_align("hi");
        assert_eq!(visible_len(&aligned), 10);
        assert_eq!(aligned, "        hi");
    }

    #[test]
    fn alignment_uses_display_width_for_styled_cjk() {
        let styled = Style::new().fg(Color::Green).render("中");
        let centered = Paragraph::new("")
            .width(6)
            .align(TextAlign::Center)
            .apply_align("中");
        let right = Paragraph::new("")
            .width(6)
            .align(TextAlign::Right)
            .apply_align(&styled);

        assert_eq!(centered, "  中  ");
        assert_eq!(visible_len(&right), 6);
        assert_eq!(strip_ansi(&right), "    中");
    }

    #[test]
    fn alignment_preserves_content_when_width_is_zero() {
        let centered = Paragraph::new("hello").width(0).align(TextAlign::Center);
        let right = Paragraph::new("hello").width(0).align(TextAlign::Right);

        assert_eq!(centered.apply_align("hello"), "hello");
        assert_eq!(right.apply_align("hello"), "hello");
    }

    #[test]
    fn wraps_cjk_by_display_width() {
        let p = Paragraph::new("中文测试内容").width(8);
        let lines = p.wrap();

        assert!(lines.iter().all(|line| visible_len(line) <= 8));
        assert_eq!(lines.concat(), "中文测试内容");
    }

    #[test]
    fn lines_apply_alignment_and_color_for_string_rendering() {
        let lines = Paragraph::new("hi")
            .width(6)
            .align(TextAlign::Right)
            .color(Color::Green)
            .lines();

        assert_eq!(lines.len(), 1);
        assert_eq!(strip_ansi(&lines[0]), "    hi");
        assert_eq!(visible_len(&lines[0]), 6);
        assert!(lines[0].contains("\x1b[32m"));
    }

    #[test]
    fn view_joins_wrapped_lines() {
        let rendered = Paragraph::new("alpha beta gamma").width(8).view();
        let plain = strip_ansi(&rendered);

        assert!(plain.contains('\n'));
        assert!(plain.lines().all(|line| visible_len(line) <= 8));
    }

    #[test]
    fn element_produces_box() {
        let p = Paragraph::new("test text").width(20);
        let el: Element<()> = p.element();
        match el {
            Element::Box(b) => assert!(!b.children.is_empty()),
            _ => panic!("expected Box"),
        }
    }
}
