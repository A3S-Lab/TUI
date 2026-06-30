use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{visible_len, wrap_words, Color};

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
        self.width = w;
        self
    }

    pub fn indent(mut self, n: usize) -> Self {
        self.indent = n;
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
        let effective_width = self.width.saturating_sub(self.indent);
        if effective_width == 0 {
            return vec![self.text.clone()];
        }

        let indent_str = " ".repeat(self.indent);
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
        let available = self.width;

        match self.align {
            TextAlign::Left => line.to_string(),
            TextAlign::Center => {
                let pad = available.saturating_sub(content_len) / 2;
                format!("{}{}", " ".repeat(pad), content)
            }
            TextAlign::Right => {
                let pad = available.saturating_sub(content_len);
                format!("{}{}", " ".repeat(pad), content)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn indent_adds_spaces() {
        let p = Paragraph::new("hello").width(80).indent(4);
        let lines = p.wrap();
        assert!(lines[0].starts_with("    "));
    }

    #[test]
    fn center_alignment() {
        let p = Paragraph::new("hi").width(10).align(TextAlign::Center);
        let aligned = p.apply_align("hi");
        assert!(aligned.starts_with("    "));
    }

    #[test]
    fn right_alignment() {
        let p = Paragraph::new("hi").width(10).align(TextAlign::Right);
        let aligned = p.apply_align("hi");
        assert!(aligned.starts_with("        "));
    }

    #[test]
    fn wraps_cjk_by_display_width() {
        let p = Paragraph::new("中文测试内容").width(8);
        let lines = p.wrap();

        assert!(lines.iter().all(|line| visible_len(line) <= 8));
        assert_eq!(lines.concat(), "中文测试内容");
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
