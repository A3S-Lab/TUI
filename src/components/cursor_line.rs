use crate::element::{BoxElement, Element, TextElement};
use crate::style::{next_display_cell_boundary, slice_visible_cols, visible_len, Color, Style};

const MAX_CURSOR_LINE_WIDTH: usize = u16::MAX as usize;

/// A horizontally scrollable single text line with a block cursor.
///
/// `cursor_col` and `scroll_col` are terminal display columns, not byte or char
/// offsets, so callers can use it with CJK/wide text and ANSI-stripped editor
/// buffers.
#[derive(Debug, Clone)]
pub struct CursorLine {
    text: String,
    cursor_col: usize,
    scroll_col: usize,
    width: Option<usize>,
    cursor_style: Style,
    fill_width: bool,
}

impl CursorLine {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            cursor_col: 0,
            scroll_col: 0,
            width: None,
            cursor_style: Style::new().reverse(),
            fill_width: false,
        }
    }

    pub fn cursor_col(mut self, col: usize) -> Self {
        self.cursor_col = col;
        self
    }

    pub fn scroll_col(mut self, col: usize) -> Self {
        self.scroll_col = col;
        self
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width.min(MAX_CURSOR_LINE_WIDTH));
        self
    }

    pub fn cursor_style(mut self, style: Style) -> Self {
        self.cursor_style = style;
        self
    }

    pub fn fill_width(mut self, fill: bool) -> Self {
        self.fill_width = fill;
        self
    }

    pub fn view(&self) -> String {
        let parts = self.parts();
        if let Some(cursor) = parts.cursor {
            format!(
                "{}{}{}",
                parts.before,
                self.cursor_style.render(&cursor),
                parts.after
            )
        } else {
            format!("{}{}", parts.before, parts.after)
        }
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let parts = self.parts();
        if let Some(cursor) = parts.cursor {
            Element::Box(
                BoxElement::row()
                    .child(Element::Text(TextElement::new(parts.before)))
                    .child(Element::Text(
                        self.apply_cursor_style(TextElement::new(cursor)),
                    ))
                    .child(Element::Text(TextElement::new(parts.after))),
            )
        } else {
            Element::Text(TextElement::new(format!("{}{}", parts.before, parts.after)))
        }
    }

    fn cursor_visible(&self) -> bool {
        if self.cursor_col < self.scroll_col {
            return false;
        }

        match self.width {
            Some(width) => self.cursor_col < self.scroll_col.saturating_add(width),
            None => true,
        }
    }

    fn parts(&self) -> CursorLineParts {
        let end_col = self
            .width
            .map(|width| self.scroll_col.saturating_add(width))
            .unwrap_or(usize::MAX);
        let window = slice_visible_cols(&self.text, self.scroll_col, end_col);

        let (before, cursor, mut after) = if self.cursor_visible() {
            let rel_col = self.cursor_col.saturating_sub(self.scroll_col);
            let (before, cursor, after) = split_window_at_cursor(&window, rel_col);
            (before, Some(cursor), after)
        } else {
            (window, None, String::new())
        };

        if self.fill_width {
            if let Some(width) = self.width {
                let current = match cursor.as_ref() {
                    Some(cursor) => {
                        visible_len(&before) + visible_len(cursor) + visible_len(&after)
                    }
                    None => visible_len(&before) + visible_len(&after),
                };
                if current < width {
                    after.push_str(&" ".repeat(width - current));
                }
            }
        }

        CursorLineParts {
            before,
            cursor,
            after,
        }
    }

    fn apply_cursor_style(&self, mut text: TextElement) -> TextElement {
        if let Some(fg) = self.cursor_style.foreground() {
            text = text.fg(fg);
        }
        if let Some(bg) = self.cursor_style.background() {
            text = text.bg(bg);
        }
        if self.cursor_style.is_bold() {
            text = text.bold();
        }
        if self.cursor_style.is_italic() {
            text = text.italic();
        }
        if self.cursor_style.is_underline() {
            text = text.underline();
        }
        if self.cursor_style.is_dim() {
            text = text.dim();
        }
        if self.cursor_style.is_strikethrough() {
            text = text.strikethrough();
        }
        if self.cursor_style.is_reverse() {
            text = text.reverse();
            if self.cursor_style.foreground().is_none() && self.cursor_style.background().is_none()
            {
                text = text.fg(Color::Black).bg(Color::White);
            }
        }

        text
    }
}

struct CursorLineParts {
    before: String,
    cursor: Option<String>,
    after: String,
}

fn split_window_at_cursor(window: &str, cursor_col: usize) -> (String, String, String) {
    let mut before = String::new();
    let mut col = 0usize;
    let mut index = 0usize;

    while let Some((end, width)) = next_display_cell_boundary(window, index) {
        let cell = &window[index..end];
        if width == 0 {
            before.push_str(cell);
            index = end;
            continue;
        }

        let next_col = col.saturating_add(width);
        if cursor_col < next_col {
            return (before, cell.to_string(), window[end..].to_string());
        }

        before.push_str(cell);
        col = next_col;
        index = end;
    }

    (window.to_string(), " ".to_string(), String::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len, Color};

    #[test]
    fn renders_block_cursor_at_display_column() {
        let line = CursorLine::new("hello")
            .cursor_col(1)
            .cursor_style(Style::new().fg(Color::Black).bg(Color::Blue))
            .view();

        assert_eq!(strip_ansi(&line), "hello");
        assert!(line.contains("\x1b[30;44me\x1b[0m"));
    }

    #[test]
    fn renders_cursor_space_past_end_of_short_line() {
        let line = CursorLine::new("hi").cursor_col(2).view();

        assert_eq!(strip_ansi(&line), "hi ");
        assert!(line.contains("\x1b[7m \x1b[0m"));
    }

    #[test]
    fn scrolls_by_display_columns_for_cjk_text() {
        let line = CursorLine::new("ab你好cd")
            .scroll_col(2)
            .width(4)
            .cursor_col(4)
            .cursor_style(Style::new().bg(Color::Yellow))
            .view();

        assert_eq!(strip_ansi(&line), "你好");
        assert!(line.contains("好"));
        assert!(line.contains("\x1b[43m"));
    }

    #[test]
    fn cursor_inside_wide_glyph_styles_whole_glyph() {
        let line = CursorLine::new("你好").cursor_col(1).view();

        assert_eq!(strip_ansi(&line), "你好");
        assert!(line.contains("\x1b[7m你\x1b[0m"));
    }

    #[test]
    fn cursor_styles_following_zero_width_marks_with_base_glyph() {
        let line = CursorLine::new("e\u{301}x").cursor_col(0).view();

        assert_eq!(strip_ansi(&line), "e\u{301}x");
        assert!(line.contains("\x1b[7me\u{301}\x1b[0mx"));
    }

    #[test]
    fn does_not_render_cursor_outside_visible_window() {
        let line = CursorLine::new("abcdef")
            .scroll_col(2)
            .width(3)
            .cursor_col(1)
            .view();

        assert_eq!(line, "cde");
        assert!(!line.contains('\u{1b}'));
    }

    #[test]
    fn fill_width_pads_visible_width() {
        let line = CursorLine::new("x")
            .cursor_col(1)
            .width(4)
            .fill_width(true)
            .view();

        assert_eq!(visible_len(&line), 4);
        assert_eq!(strip_ansi(&line), "x   ");
    }

    #[test]
    fn oversized_width_is_clamped() {
        let line = CursorLine::new("x")
            .cursor_col(1)
            .width(usize::MAX)
            .fill_width(true);
        let rendered = line.view();

        assert_eq!(line.width, Some(MAX_CURSOR_LINE_WIDTH));
        assert_eq!(visible_len(&rendered), MAX_CURSOR_LINE_WIDTH);
    }

    #[test]
    fn oversized_cursor_column_does_not_overflow() {
        let line = CursorLine::new("abc").cursor_col(usize::MAX).view();

        assert_eq!(strip_ansi(&line), "abc ");
    }

    #[test]
    fn element_contains_rendered_cursor_line() {
        let element: Element<()> = CursorLine::new("abc").cursor_col(0).element();

        match element {
            Element::Box(row) => {
                assert_eq!(row.children.len(), 3);
                match &row.children[1] {
                    Element::Text(text) => {
                        assert_eq!(text.content, "a");
                        assert_eq!(text.style.fg, Some(Color::Black));
                        assert_eq!(text.style.bg, Some(Color::White));
                        assert!(text.style.reverse);
                    }
                    _ => panic!("expected cursor text element"),
                }
            }
            _ => panic!("expected row element"),
        }
    }
}
