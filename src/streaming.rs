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
