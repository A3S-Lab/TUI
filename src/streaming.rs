//! Streaming markdown renderer for real-time content (e.g., LLM token output).

use crate::element::Element;
use crate::markdown::Markdown;

pub struct StreamingMarkdown {
    buffer: String,
    rendered_up_to: usize,
    rendered_lines: Vec<String>,
    md: Markdown,
}

impl StreamingMarkdown {
    pub fn new(width: usize) -> Self {
        Self {
            buffer: String::new(),
            rendered_up_to: 0,
            rendered_lines: Vec::new(),
            md: Markdown::new().with_width(width),
        }
    }

    pub fn push(&mut self, token: &str) {
        self.buffer.push_str(token);
        self.rerender();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.rendered_up_to = 0;
        self.rendered_lines.clear();
    }

    pub fn view(&self) -> String {
        self.rendered_lines.join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        self.md.render_element(&self.buffer)
    }

    pub fn line_count(&self) -> usize {
        self.rendered_lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn raw_content(&self) -> &str {
        &self.buffer
    }

    fn rerender(&mut self) {
        let rendered = self.md.render(&self.buffer);
        self.rendered_lines = rendered.lines().map(|l| l.to_string()).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_initially() {
        let sm = StreamingMarkdown::new(80);
        assert!(sm.is_empty());
        assert_eq!(sm.line_count(), 0);
        assert_eq!(sm.raw_content(), "");
    }

    #[test]
    fn push_accumulates() {
        let mut sm = StreamingMarkdown::new(80);
        sm.push("Hello");
        sm.push(" world");
        assert_eq!(sm.raw_content(), "Hello world");
        assert!(!sm.is_empty());
    }

    #[test]
    fn clear_resets() {
        let mut sm = StreamingMarkdown::new(80);
        sm.push("some content");
        sm.clear();
        assert!(sm.is_empty());
        assert_eq!(sm.line_count(), 0);
    }

    #[test]
    fn multiline_content() {
        let mut sm = StreamingMarkdown::new(80);
        sm.push("line1\n\nline2");
        assert!(sm.line_count() >= 2);
    }

    #[test]
    fn view_returns_rendered() {
        let mut sm = StreamingMarkdown::new(80);
        sm.push("hello");
        let view = sm.view();
        assert!(!view.is_empty());
    }
}
