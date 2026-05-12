use crate::style::visible_len;

pub struct Viewport {
    lines: Vec<String>,
    offset: usize,
    width: u16,
    height: u16,
    auto_scroll: bool,
}

#[derive(Debug, Clone)]
pub enum ViewportMsg {
    ScrollUp(usize),
    ScrollDown(usize),
    PageUp,
    PageDown,
    Top,
    Bottom,
}

impl Viewport {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            lines: Vec::new(),
            offset: 0,
            width,
            height,
            auto_scroll: true,
        }
    }

    pub fn with_auto_scroll(mut self, auto: bool) -> Self {
        self.auto_scroll = auto;
        self
    }

    pub fn set_content(&mut self, content: &str) {
        self.lines = self.wrap_content(content);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn append(&mut self, content: &str) {
        let new_lines = self.wrap_content(content);
        self.lines.extend(new_lines);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.offset = 0;
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        let raw: String = self.lines.join("\n");
        self.lines = self.wrap_content(&raw);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn update(&mut self, msg: ViewportMsg) {
        match msg {
            ViewportMsg::ScrollUp(n) => {
                self.offset = self.offset.saturating_sub(n);
            }
            ViewportMsg::ScrollDown(n) => {
                self.offset = (self.offset + n).min(self.max_offset());
            }
            ViewportMsg::PageUp => {
                self.offset = self.offset.saturating_sub(self.height as usize);
            }
            ViewportMsg::PageDown => {
                self.offset =
                    (self.offset + self.height as usize).min(self.max_offset());
            }
            ViewportMsg::Top => {
                self.offset = 0;
            }
            ViewportMsg::Bottom => {
                self.scroll_to_bottom();
            }
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.offset = self.max_offset();
    }

    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    pub fn scroll_percent(&self) -> u8 {
        if self.lines.len() <= self.height as usize {
            return 100;
        }
        let max = self.max_offset();
        if max == 0 {
            return 100;
        }
        ((self.offset as f64 / max as f64) * 100.0) as u8
    }

    pub fn at_bottom(&self) -> bool {
        self.offset >= self.max_offset()
    }

    pub fn view(&self) -> String {
        let h = self.height as usize;
        let end = (self.offset + h).min(self.lines.len());
        let visible: Vec<&str> = self.lines[self.offset..end]
            .iter()
            .map(|s| s.as_str())
            .collect();

        let mut result = visible.join("\n");

        let visible_count = end - self.offset;
        if visible_count < h {
            for _ in 0..(h - visible_count) {
                result.push('\n');
            }
        }

        result
    }

    fn max_offset(&self) -> usize {
        self.lines.len().saturating_sub(self.height as usize)
    }

    fn wrap_content(&self, content: &str) -> Vec<String> {
        let w = self.width as usize;
        let mut result = Vec::new();

        for line in content.lines() {
            if w == 0 {
                result.push(line.to_string());
                continue;
            }
            let vis = visible_len(line);
            if vis <= w {
                result.push(line.to_string());
            } else {
                let wrapped = wrap_line(line, w);
                result.extend(wrapped);
            }
        }

        if content.ends_with('\n') && !content.is_empty() {
            result.push(String::new());
        }

        result
    }
}

fn wrap_line(s: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    let mut in_escape = false;

    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
            current.push(c);
            continue;
        }
        if in_escape {
            current.push(c);
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
            continue;
        }

        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if current_width + cw > width && current_width > 0 {
            lines.push(current);
            current = String::new();
            current_width = 0;
        }
        current.push(c);
        current_width += cw;
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}
