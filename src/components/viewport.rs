use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{MouseEvent, MouseEventKind};
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

    /// Pause/resume auto-follow (e.g. when the user scrolls up mid-stream).
    pub fn set_auto_scroll(&mut self, auto: bool) {
        self.auto_scroll = auto;
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
                self.offset = (self.offset + self.height as usize).min(self.max_offset());
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

    /// Handle mouse scroll events.
    pub fn handle_mouse(&mut self, mouse: &MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.update(ViewportMsg::ScrollUp(3));
            }
            MouseEventKind::ScrollDown => {
                self.update(ViewportMsg::ScrollDown(3));
            }
            _ => {}
        }
    }

    pub fn view(&self) -> String {
        let h = self.height as usize;
        // Clamp the offset: after a resize re-wraps to fewer lines, a stale
        // offset would slice past the end and blank the whole transcript.
        let max_off = self.lines.len().saturating_sub(h);
        let offset = self.offset.min(max_off);
        let end = (offset + h).min(self.lines.len());
        let visible: Vec<&str> = self.lines[offset..end].iter().map(|s| s.as_str()).collect();

        let mut result = visible.join("\n");

        let visible_count = end - offset;
        if visible_count < h {
            for _ in 0..(h - visible_count) {
                result.push('\n');
            }
        }

        result
    }

    /// Render visible lines as an Element tree.
    pub fn element<Msg>(&self) -> Element<Msg> {
        let h = self.height as usize;
        let end = (self.offset + h).min(self.lines.len());

        let mut children: Vec<Element<Msg>> = self.lines[self.offset..end]
            .iter()
            .map(|line| Element::Text(TextElement::new(line.as_str())))
            .collect();

        let visible_count = end - self.offset;
        for _ in visible_count..h {
            children.push(Element::Text(TextElement::new("")));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
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
    // The gutter prefixes content with leading spaces (the left margin); keep
    // that indent on wrapped continuation lines so they don't fall back to the
    // screen edge.
    let indent = s
        .chars()
        .take_while(|c| *c == ' ')
        .count()
        .min(width.saturating_sub(8));
    let pad = " ".repeat(indent);
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
            current = pad.clone();
            current_width = indent;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_content_and_view() {
        let mut vp = Viewport::new(80, 3);
        vp.set_content("line1\nline2\nline3");
        let view = vp.view();
        assert!(view.contains("line1"));
        assert!(view.contains("line3"));
    }

    #[test]
    fn stale_offset_after_shrink_does_not_blank_or_panic() {
        // Regression: scroll up, then resize to far fewer lines. A stale offset
        // must clamp instead of slicing past the end (which blanked the screen).
        let mut vp = Viewport::new(80, 5).with_auto_scroll(false);
        vp.set_content(&(1..=50).map(|i| i.to_string()).collect::<Vec<_>>().join("\n"));
        vp.update(ViewportMsg::ScrollDown(40)); // offset deep in the content
        vp.set_content("only\ntwo"); // now far fewer lines
        let view = vp.view();
        assert!(view.contains("only"), "clamped offset still shows content");
    }

    #[test]
    fn wrapped_line_keeps_leading_indent() {
        // A gutter-indented long line wraps; continuations keep the indent
        // instead of falling back to column 0. (Width must exceed indent+8, or
        // the indent is capped to keep usable wrap width.)
        let mut vp = Viewport::new(40, 5);
        vp.set_content(&format!("    {}", "x".repeat(60)));
        let view = vp.view();
        let wrapped: Vec<&str> = view.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(wrapped.len() >= 2, "long line wrapped");
        assert!(wrapped[1].starts_with("    "), "continuation keeps indent");
    }

    #[test]
    fn scroll_down() {
        let mut vp = Viewport::new(80, 2).with_auto_scroll(false);
        vp.set_content("a\nb\nc\nd\ne");
        assert_eq!(vp.total_lines(), 5);
        vp.update(ViewportMsg::ScrollDown(2));
        let view = vp.view();
        assert!(view.contains("c"));
    }

    #[test]
    fn scroll_up() {
        let mut vp = Viewport::new(80, 2).with_auto_scroll(false);
        vp.set_content("a\nb\nc\nd");
        vp.update(ViewportMsg::ScrollDown(3));
        vp.update(ViewportMsg::ScrollUp(1));
        let view = vp.view();
        assert!(view.contains("c"));
    }

    #[test]
    fn auto_scroll_to_bottom() {
        let mut vp = Viewport::new(80, 2);
        vp.set_content("a\nb\nc\nd\ne");
        assert!(vp.at_bottom());
    }

    #[test]
    fn page_up_down() {
        let mut vp = Viewport::new(80, 2).with_auto_scroll(false);
        vp.set_content("1\n2\n3\n4\n5\n6\n7\n8");
        vp.update(ViewportMsg::PageDown);
        vp.update(ViewportMsg::PageDown);
        vp.update(ViewportMsg::PageUp);
        let view = vp.view();
        assert!(view.contains("3"));
    }

    #[test]
    fn top_and_bottom() {
        let mut vp = Viewport::new(80, 2).with_auto_scroll(false);
        vp.set_content("a\nb\nc\nd\ne");
        vp.update(ViewportMsg::Bottom);
        assert!(vp.at_bottom());
        vp.update(ViewportMsg::Top);
        assert_eq!(vp.scroll_percent(), 0);
    }

    #[test]
    fn append_content() {
        let mut vp = Viewport::new(80, 3);
        vp.set_content("first");
        vp.append("\nsecond");
        assert!(vp.total_lines() >= 2);
        let view = vp.view();
        assert!(view.contains("second"));
    }

    #[test]
    fn clear_resets() {
        let mut vp = Viewport::new(80, 3);
        vp.set_content("hello\nworld");
        vp.clear();
        assert_eq!(vp.total_lines(), 0);
    }
}
