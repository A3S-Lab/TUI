//! Streaming markdown renderer for real-time content (e.g., LLM token output).

use crate::element::Element;
use crate::markdown::{rendered_markdown_element, split_rendered_lines, Markdown};

pub struct StreamingMarkdown {
    buffer: String,
    rendered: String,
    rendered_line_count: usize,
    md: Markdown,
}

impl StreamingMarkdown {
    pub fn new(width: usize) -> Self {
        Self {
            buffer: String::new(),
            rendered: String::new(),
            rendered_line_count: 0,
            md: Markdown::new().with_width(width),
        }
    }

    pub fn push(&mut self, token: &str) {
        self.buffer.push_str(token);
        self.rerender();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.rendered.clear();
        self.rendered_line_count = 0;
    }

    pub fn view(&self) -> String {
        self.rendered.clone()
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        rendered_markdown_element(&self.rendered)
    }

    pub fn line_count(&self) -> usize {
        self.rendered_line_count
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn raw_content(&self) -> &str {
        &self.buffer
    }

    fn rerender(&mut self) {
        self.rendered = self.md.render(&self.buffer);
        self.rendered_line_count = split_rendered_lines(&self.rendered).len();
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

    #[test]
    fn preserves_trailing_markdown_blank_rows() {
        let mut sm = StreamingMarkdown::new(80);
        sm.push("# Hello");

        assert_eq!(sm.line_count(), 2);
        assert!(sm.view().ends_with('\n'));
    }

    #[test]
    fn element_uses_the_cached_rendered_output() {
        let mut sm = StreamingMarkdown::new(80);
        sm.push("**cached**");

        // Simulate source advancing without a render commit. `element()` must
        // consume the cached output from the last `push`, never parse `buffer`.
        sm.buffer.clear();
        sm.buffer.push_str("uncached");

        let Element::Box(column) = sm.element::<()>() else {
            panic!("expected streaming Markdown column");
        };
        let [Element::Text(text)] = column.children.as_slice() else {
            panic!("expected one cached rendered row");
        };
        assert_eq!(text.content, "cached");
        assert!(text.style.bold);
    }
}
