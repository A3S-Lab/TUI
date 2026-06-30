use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::KeyEvent;
use crate::style::Color;
use crossterm::event::{KeyCode, KeyModifiers};

pub struct Textarea {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    offset: usize,
    width: u16,
    height: u16,
    placeholder: String,
    focused: bool,
    char_limit: Option<usize>,
    submit_on_enter: bool,
    /// When set, the height auto-fits the line count, clamped to this max.
    auto_grow_max: Option<u16>,
}

#[derive(Debug, Clone)]
pub enum TextareaMsg {
    Changed(String),
    Submit(String),
}

impl Textarea {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            offset: 0,
            width: 80,
            height: 5,
            placeholder: String::new(),
            focused: true,
            char_limit: None,
            submit_on_enter: false,
            auto_grow_max: None,
        }
    }

    pub fn with_placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }

    pub fn with_height(mut self, h: u16) -> Self {
        self.height = h;
        self
    }

    /// Auto-fit the visible height to the number of lines (1..=max), so the box
    /// grows with embedded newlines instead of scrolling a single row.
    pub fn with_auto_grow(mut self, max: u16) -> Self {
        self.auto_grow_max = Some(max.max(1));
        self.fit_height();
        self
    }

    /// Current visible height (rows). Grows with content when auto-grow is on.
    pub fn height(&self) -> u16 {
        self.height
    }

    fn fit_height(&mut self) {
        if let Some(max) = self.auto_grow_max {
            let n = self.lines.len() as u16;
            self.height = n.clamp(1, max);
            // The box grew to fit every line, so the internal scroll offset (set
            // while the height was smaller, to follow the cursor) must reset —
            // otherwise earlier lines stay scrolled out of view.
            if n <= max {
                self.offset = 0;
            }
        }
    }

    pub fn with_width(mut self, w: u16) -> Self {
        self.width = w;
        self
    }

    pub fn with_char_limit(mut self, limit: usize) -> Self {
        self.char_limit = Some(limit);
        self
    }

    pub fn with_submit_on_enter(mut self, submit: bool) -> Self {
        self.submit_on_enter = submit;
        self
    }

    pub fn focus(&mut self) {
        self.focused = true;
    }
    pub fn blur(&mut self) {
        self.focused = false;
    }

    pub fn value(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_value(&mut self, v: &str) {
        self.lines = v.lines().map(|l| l.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = self.lines.len() - 1;
        self.cursor_col = Self::char_len(&self.lines[self.cursor_row]);
        self.fit_height();
    }

    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.offset = 0;
        self.fit_height();
    }

    pub fn total_chars(&self) -> usize {
        self.lines.iter().map(|l| l.chars().count()).sum::<usize>() + self.lines.len() - 1
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<TextareaMsg> {
        if !self.focused {
            return None;
        }

        let result = match (key.code, key.modifiers) {
            (KeyCode::Enter, KeyModifiers::NONE) if self.submit_on_enter => {
                Some(TextareaMsg::Submit(self.value()))
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                self.insert_newline();
                Some(TextareaMsg::Changed(self.value()))
            }
            // Shift/Alt+Enter inserts a newline instead of submitting (works in
            // terminals that report the modifier; some fold it into plain Enter).
            (KeyCode::Enter, m)
                if m.contains(KeyModifiers::SHIFT) || m.contains(KeyModifiers::ALT) =>
            {
                self.insert_newline();
                Some(TextareaMsg::Changed(self.value()))
            }
            (KeyCode::Char('j'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.insert_newline();
                Some(TextareaMsg::Changed(self.value()))
            }
            (KeyCode::Char('a'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.cursor_col = 0;
                None
            }
            (KeyCode::Char('e'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.cursor_col = Self::char_len(&self.lines[self.cursor_row]);
                None
            }
            (KeyCode::Char('k'), m) if m.contains(KeyModifiers::CONTROL) => {
                let off = Self::byte_off(&self.lines[self.cursor_row], self.cursor_col);
                self.lines[self.cursor_row].truncate(off);
                Some(TextareaMsg::Changed(self.value()))
            }
            (KeyCode::Char(c), _) => {
                if let Some(limit) = self.char_limit {
                    if self.total_chars() >= limit {
                        return None;
                    }
                }
                self.insert_char(c);
                Some(TextareaMsg::Changed(self.value()))
            }
            (KeyCode::Backspace, _) => {
                if self.delete_backward() {
                    Some(TextareaMsg::Changed(self.value()))
                } else {
                    None
                }
            }
            (KeyCode::Delete, _) => {
                if self.delete_forward() {
                    Some(TextareaMsg::Changed(self.value()))
                } else {
                    None
                }
            }
            (KeyCode::Left, _) => {
                self.move_left();
                None
            }
            (KeyCode::Right, _) => {
                self.move_right();
                None
            }
            (KeyCode::Up, _) => {
                self.move_up();
                None
            }
            (KeyCode::Down, _) => {
                self.move_down();
                None
            }
            (KeyCode::Home, _) => {
                self.cursor_col = 0;
                None
            }
            (KeyCode::End, _) => {
                self.cursor_col = Self::char_len(&self.lines[self.cursor_row]);
                None
            }
            _ => None,
        };
        self.fit_height();
        result
    }

    pub fn view(&self) -> String {
        if self.lines == vec![String::new()] && !self.placeholder.is_empty() && !self.focused {
            return format!("\x1b[2m{}\x1b[0m", self.placeholder);
        }

        let h = self.height as usize;
        let end = (self.offset + h).min(self.lines.len());
        let visible = &self.lines[self.offset..end];

        let mut result = Vec::new();
        for (i, line) in visible.iter().enumerate() {
            let row = self.offset + i;
            if row == self.cursor_row && self.focused {
                result.push(self.render_line_with_cursor(line));
            } else {
                result.push(line.clone());
            }
        }

        for _ in result.len()..h {
            result.push("~".to_string());
        }

        result.join("\n")
    }

    fn char_width(c: char) -> usize {
        unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
    }

    /// First visible display column of the cursor's line (horizontal scroll), so
    /// the insertion point stays on screen. CJK glyphs count as 2 columns.
    fn scroll_start(&self) -> usize {
        let width = (self.width as usize).max(1);
        let cursor_disp = self.cursor_display_col_abs();
        cursor_disp.saturating_sub(width.saturating_sub(1))
    }

    /// Absolute display column of the insertion point within its line.
    fn cursor_display_col_abs(&self) -> usize {
        self.lines
            .get(self.cursor_row)
            .map(|line| {
                line.chars()
                    .take(self.cursor_col)
                    .map(Self::char_width)
                    .sum()
            })
            .unwrap_or(0)
    }

    /// Display column of the insertion point relative to the visible window —
    /// what a host needs to place the real terminal cursor.
    pub fn cursor_display_col(&self) -> usize {
        self.cursor_display_col_abs() - self.scroll_start()
    }

    /// The cursor's row within the visible window (for multi-line input — the
    /// host places the real terminal cursor on this row).
    pub fn cursor_row(&self) -> usize {
        self.cursor_row.saturating_sub(self.offset)
    }

    /// Render the cursor's line as plain (horizontally scrolled) text; the real
    /// terminal cursor marks the insertion point, so no reverse-video block here.
    fn render_line_with_cursor(&self, line: &str) -> String {
        let width = (self.width as usize).max(1);
        let start = self.scroll_start();

        let mut out = String::new();
        let mut disp = 0usize; // absolute display column scanned
        let mut shown = 0usize; // visible columns emitted
        for ch in line.chars() {
            let w = Self::char_width(ch);
            if disp < start {
                disp += w; // starts before the scroll window — skip wholly
                continue;
            }
            if shown + w > width {
                break; // right edge reached
            }
            out.push(ch);
            disp += w;
            shown += w;
        }
        out
    }

    pub fn set_width(&mut self, w: u16) {
        self.width = w;
    }

    // cursor_col is a CHAR index; convert to a byte offset before String ops.
    fn byte_off(line: &str, col: usize) -> usize {
        line.char_indices().nth(col).map_or(line.len(), |(b, _)| b)
    }

    fn char_len(line: &str) -> usize {
        line.chars().count()
    }

    /// Insert a (possibly multi-line) string at the cursor — used for paste, so
    /// newlines become real line breaks instead of submitting the message.
    pub fn insert_str(&mut self, text: &str) {
        for ch in text.chars() {
            match ch {
                '\n' => self.insert_newline(),
                '\r' => {} // drop CR so CRLF pastes don't double-break
                _ => self.insert_char(ch),
            }
        }
    }

    fn insert_char(&mut self, c: char) {
        let off = Self::byte_off(&self.lines[self.cursor_row], self.cursor_col);
        self.lines[self.cursor_row].insert(off, c);
        self.cursor_col += 1;
    }

    fn insert_newline(&mut self) {
        let off = Self::byte_off(&self.lines[self.cursor_row], self.cursor_col);
        let rest = self.lines[self.cursor_row].split_off(off);
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;
        self.ensure_visible();
    }

    fn delete_backward(&mut self) -> bool {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            let off = Self::byte_off(&self.lines[self.cursor_row], self.cursor_col);
            self.lines[self.cursor_row].remove(off);
            true
        } else if self.cursor_row > 0 {
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = Self::char_len(&self.lines[self.cursor_row]);
            self.lines[self.cursor_row].push_str(&current_line);
            self.ensure_visible();
            true
        } else {
            false
        }
    }

    fn delete_forward(&mut self) -> bool {
        if self.cursor_col < Self::char_len(&self.lines[self.cursor_row]) {
            let off = Self::byte_off(&self.lines[self.cursor_row], self.cursor_col);
            self.lines[self.cursor_row].remove(off);
            true
        } else if self.cursor_row + 1 < self.lines.len() {
            let next_line = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next_line);
            true
        } else {
            false
        }
    }

    fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = Self::char_len(&self.lines[self.cursor_row]);
            self.ensure_visible();
        }
    }

    fn move_right(&mut self) {
        if self.cursor_col < Self::char_len(&self.lines[self.cursor_row]) {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
            self.ensure_visible();
        }
    }

    fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self
                .cursor_col
                .min(Self::char_len(&self.lines[self.cursor_row]));
            self.ensure_visible();
        }
    }

    fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = self
                .cursor_col
                .min(Self::char_len(&self.lines[self.cursor_row]));
            self.ensure_visible();
        }
    }

    fn ensure_visible(&mut self) {
        let h = self.height as usize;
        if self.cursor_row < self.offset {
            self.offset = self.cursor_row;
        } else if self.cursor_row >= self.offset + h {
            self.offset = self.cursor_row - h + 1;
        }
    }
}

impl Default for Textarea {
    fn default() -> Self {
        Self::new()
    }
}

impl Textarea {
    pub fn element<Msg>(&self) -> Element<Msg> {
        if self.lines == vec![String::new()] && !self.placeholder.is_empty() && !self.focused {
            return Element::Text(
                TextElement::new(&self.placeholder)
                    .dim()
                    .fg(Color::BrightBlack),
            );
        }

        let h = self.height as usize;
        let end = (self.offset + h).min(self.lines.len());
        let visible = &self.lines[self.offset..end];

        let mut children: Vec<Element<Msg>> = Vec::new();
        for (i, line) in visible.iter().enumerate() {
            let row = self.offset + i;
            if row == self.cursor_row && self.focused {
                children.push(Element::Text(TextElement::new(
                    self.render_line_with_cursor(line),
                )));
            } else {
                children.push(Element::Text(TextElement::new(line.as_str())));
            }
        }

        for _ in children.len()..h {
            children.push(Element::Text(
                TextElement::new("~").dim().fg(Color::BrightBlack),
            ));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }

    #[test]
    fn typing_text() {
        let mut ta = Textarea::new();
        ta.handle_key(&key(KeyCode::Char('h')));
        ta.handle_key(&key(KeyCode::Char('i')));
        assert_eq!(ta.value(), "hi");
    }

    #[test]
    fn multibyte_input_no_panic() {
        // Regression: cursor_col mixed char/byte indexing → panic on CJK input.
        let mut ta = Textarea::new();
        for c in "你好abc世界".chars() {
            ta.handle_key(&key(KeyCode::Char(c)));
        }
        assert_eq!(ta.value(), "你好abc世界");
        // Move left over multibyte chars and edit — must not panic on a boundary.
        // chars: 你 好 a b c 世 界 ; cursor at end (7), Left x4 -> col 3 (before 'b')
        for _ in 0..4 {
            ta.handle_key(&key(KeyCode::Left));
        }
        ta.handle_key(&key(KeyCode::Backspace)); // col 3->2, deletes 'a'
        ta.handle_key(&key(KeyCode::Delete)); // deletes 'b' at col 2
        assert_eq!(ta.value(), "你好c世界");
        let _ = ta.view();
    }

    #[test]
    fn cjk_cursor_stays_in_view_when_line_overflows() {
        // Narrow box; type a long CJK line. The visible render must never exceed
        // the width and must always include the cursor block at the end.
        let mut ta = Textarea::new().with_width(8).with_height(1);
        for c in "你好世界你好世界".chars() {
            ta.handle_key(&key(KeyCode::Char(c)));
        }
        let line = ta.view();
        let visible = crate::style::visible_len(&line); // ANSI-stripped display width
        assert!(visible <= 8, "rendered width {visible} exceeds box width 8");
        // Insertion point must stay within the visible window for the host to
        // place the real cursor on it.
        assert!(
            ta.cursor_display_col() <= 8,
            "cursor col {} out of view",
            ta.cursor_display_col()
        );
    }

    #[test]
    fn newline_creates_new_line() {
        let mut ta = Textarea::new();
        ta.handle_key(&key(KeyCode::Char('a')));
        ta.handle_key(&key(KeyCode::Enter));
        ta.handle_key(&key(KeyCode::Char('b')));
        assert_eq!(ta.value(), "a\nb");
    }

    #[test]
    fn auto_grow_shows_all_lines_after_newline() {
        // Regression: with auto-grow, adding a newline grew the height but left
        // the scroll offset following the cursor, hiding the first line. The box
        // must show BOTH lines (height grows, offset resets).
        let mut ta = Textarea::new().with_height(1).with_auto_grow(8);
        ta.handle_key(&key(KeyCode::Char('a')));
        ta.handle_key(&key(KeyCode::Enter));
        ta.handle_key(&key(KeyCode::Char('b')));
        assert_eq!(ta.height(), 2, "box grew to two rows");
        let view = ta.view();
        assert!(view.contains('a'), "first line still visible");
        assert!(view.contains('b'), "second line visible");
    }

    #[test]
    fn backspace_joins_lines() {
        let mut ta = Textarea::new();
        ta.set_value("ab\ncd");
        ta.cursor_row = 1;
        ta.cursor_col = 0;
        ta.handle_key(&key(KeyCode::Backspace));
        assert_eq!(ta.value(), "abcd");
    }

    #[test]
    fn cursor_movement() {
        let mut ta = Textarea::new();
        ta.set_value("hello\nworld");
        ta.cursor_row = 1;
        ta.cursor_col = 5;
        ta.handle_key(&key(KeyCode::Up));
        assert_eq!(ta.cursor_row, 0);
        ta.handle_key(&key(KeyCode::Home));
        assert_eq!(ta.cursor_col, 0);
        ta.handle_key(&key(KeyCode::End));
        assert_eq!(ta.cursor_col, 5);
    }

    #[test]
    fn ctrl_a_and_e() {
        let mut ta = Textarea::new();
        ta.set_value("test");
        ta.cursor_col = 2;
        ta.handle_key(&ctrl(KeyCode::Char('a')));
        assert_eq!(ta.cursor_col, 0);
        ta.handle_key(&ctrl(KeyCode::Char('e')));
        assert_eq!(ta.cursor_col, 4);
    }

    #[test]
    fn ctrl_k_kills_line() {
        let mut ta = Textarea::new();
        ta.set_value("hello world");
        ta.cursor_col = 5;
        ta.handle_key(&ctrl(KeyCode::Char('k')));
        assert_eq!(ta.value(), "hello");
    }

    #[test]
    fn char_limit() {
        let mut ta = Textarea::new().with_char_limit(3);
        ta.handle_key(&key(KeyCode::Char('a')));
        ta.handle_key(&key(KeyCode::Char('b')));
        ta.handle_key(&key(KeyCode::Char('c')));
        ta.handle_key(&key(KeyCode::Char('d')));
        assert_eq!(ta.value(), "abc");
    }

    #[test]
    fn clear_resets() {
        let mut ta = Textarea::new();
        ta.set_value("some\ntext");
        ta.clear();
        assert_eq!(ta.value(), "");
        assert_eq!(ta.cursor_row, 0);
        assert_eq!(ta.cursor_col, 0);
    }

    #[test]
    fn submit_on_enter() {
        let mut ta = Textarea::new().with_submit_on_enter(true);
        ta.handle_key(&key(KeyCode::Char('x')));
        let msg = ta.handle_key(&key(KeyCode::Enter));
        assert!(matches!(msg, Some(TextareaMsg::Submit(s)) if s == "x"));
    }
}
