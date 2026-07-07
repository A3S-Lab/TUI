use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{Event, KeyEvent};
use crate::style::{
    display_cell_char_span, next_display_cell_boundary, previous_display_cell_char_span, Color,
};
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
            let n = self.lines.len().min(u16::MAX as usize) as u16;
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

    /// Current cursor position as `(row, char_column)`.
    pub fn cursor(&self) -> (usize, usize) {
        self.normalized_cursor()
    }

    pub fn set_value(&mut self, v: &str) {
        let lines = split_value_lines(v);
        self.lines = match self.char_limit {
            Some(limit) => clamp_lines_to_char_limit(lines, limit),
            None => lines,
        };
        self.cursor_row = self.lines.len() - 1;
        self.cursor_col = Self::char_len(&self.lines[self.cursor_row]);
        self.fit_height();
        self.ensure_visible();
    }

    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.offset = 0;
        self.fit_height();
    }

    pub fn total_chars(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line.chars().count())
            .fold(0usize, usize::saturating_add)
            .saturating_add(self.lines.len().saturating_sub(1))
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<TextareaMsg> {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Paste(text) => self.handle_paste(text),
            _ => None,
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<TextareaMsg> {
        if !self.focused {
            return None;
        }
        self.clamp_cursor();

        let result = match (key.code, key.modifiers) {
            (KeyCode::Left, m) if word_modifier(m) => {
                self.move_word_left();
                None
            }
            (KeyCode::Right, m) if word_modifier(m) => {
                self.move_word_right();
                None
            }
            (KeyCode::Char('b'), m) if m.contains(KeyModifiers::ALT) => {
                self.move_word_left();
                None
            }
            (KeyCode::Char('f'), m) if m.contains(KeyModifiers::ALT) => {
                self.move_word_right();
                None
            }
            (KeyCode::Char('w'), m) if m.contains(KeyModifiers::CONTROL) => {
                if self.delete_word_backward() {
                    Some(TextareaMsg::Changed(self.value()))
                } else {
                    None
                }
            }
            (KeyCode::Backspace, m) if word_modifier(m) => {
                if self.delete_word_backward() {
                    Some(TextareaMsg::Changed(self.value()))
                } else {
                    None
                }
            }
            (KeyCode::Delete, m) if word_modifier(m) => {
                if self.delete_word_forward() {
                    Some(TextareaMsg::Changed(self.value()))
                } else {
                    None
                }
            }
            (KeyCode::Char('d'), m) if m.contains(KeyModifiers::ALT) => {
                if self.delete_word_forward() {
                    Some(TextareaMsg::Changed(self.value()))
                } else {
                    None
                }
            }
            (KeyCode::Enter, KeyModifiers::NONE) if self.submit_on_enter => {
                Some(TextareaMsg::Submit(self.value()))
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if !self.can_insert_more() {
                    return None;
                }
                self.insert_newline();
                Some(TextareaMsg::Changed(self.value()))
            }
            // Shift/Alt+Enter inserts a newline instead of submitting (works in
            // terminals that report the modifier; some fold it into plain Enter).
            (KeyCode::Enter, m)
                if m.contains(KeyModifiers::SHIFT) || m.contains(KeyModifiers::ALT) =>
            {
                if !self.can_insert_more() {
                    return None;
                }
                self.insert_newline();
                Some(TextareaMsg::Changed(self.value()))
            }
            (KeyCode::Char('j'), m) if m.contains(KeyModifiers::CONTROL) => {
                if !self.can_insert_more() {
                    return None;
                }
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
            (KeyCode::Char(c), m) if text_char_modifier(m) => {
                if !self.can_insert_more() {
                    return None;
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

    fn handle_paste(&mut self, text: &str) -> Option<TextareaMsg> {
        if !self.focused {
            return None;
        }
        if self.insert_str_changed(text) {
            Some(TextareaMsg::Changed(self.value()))
        } else {
            None
        }
    }

    pub fn view(&self) -> String {
        let h = self.height as usize;
        if h == 0 || self.width == 0 {
            return String::new();
        }

        if self.lines == vec![String::new()] && !self.placeholder.is_empty() && !self.focused {
            return format!("\x1b[2m{}\x1b[0m", self.placeholder);
        }

        let (cursor_row, _) = self.normalized_cursor();
        let (start, end) = self.visible_range();
        let visible = &self.lines[start..end];

        let mut result = Vec::new();
        for (i, line) in visible.iter().enumerate() {
            let row = start + i;
            if row == cursor_row && self.focused {
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
        let (cursor_row, cursor_col) = self.normalized_cursor();
        self.lines
            .get(cursor_row)
            .map(|line| line.chars().take(cursor_col).map(Self::char_width).sum())
            .unwrap_or(0)
    }

    /// Display column of the insertion point relative to the visible window —
    /// what a host needs to place the real terminal cursor.
    pub fn cursor_display_col(&self) -> usize {
        self.cursor_display_col_abs()
            .saturating_sub(self.scroll_start())
    }

    /// The cursor's row within the visible window (for multi-line input — the
    /// host places the real terminal cursor on this row).
    pub fn cursor_row(&self) -> usize {
        let (cursor_row, _) = self.normalized_cursor();
        cursor_row.saturating_sub(self.normalized_offset_for_cursor(cursor_row))
    }

    /// Render the cursor's line as plain (horizontally scrolled) text; the real
    /// terminal cursor marks the insertion point, so no reverse-video block here.
    fn render_line_with_cursor(&self, line: &str) -> String {
        let width = (self.width as usize).max(1);
        let start = self.scroll_start();
        let end = start.saturating_add(width);

        let mut out = String::new();
        let mut disp = 0usize; // absolute display column scanned
        let mut shown = 0usize; // visible columns emitted
        let mut index = 0usize;
        while let Some((cell_end, w)) = next_display_cell_boundary(line, index) {
            let cell = &line[index..cell_end];
            index = cell_end;
            if w == 0 {
                if disp >= end {
                    break;
                }
                if disp >= start {
                    out.push_str(cell);
                }
                continue;
            }

            if disp >= end {
                break;
            }
            if disp < start {
                disp += w; // starts before the scroll window — skip wholly
                continue;
            }
            if shown + w > width {
                break; // right edge reached
            }
            out.push_str(cell);
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
        self.insert_str_changed(text);
    }

    fn insert_str_changed(&mut self, text: &str) -> bool {
        self.clamp_cursor();
        let mut changed = false;
        for ch in text.chars() {
            match ch {
                '\r' => {} // drop CR so CRLF pastes don't double-break
                '\n' => {
                    if !self.can_insert_more() {
                        break;
                    }
                    self.insert_newline();
                    changed = true;
                }
                '\t' => {
                    if !self.can_insert_more() {
                        break;
                    }
                    self.insert_char(' ');
                    changed = true;
                }
                ch if ch.is_control() => {}
                _ => {
                    if !self.can_insert_more() {
                        break;
                    }
                    self.insert_char(ch);
                    changed = true;
                }
            }
        }
        self.fit_height();
        self.ensure_visible();
        changed
    }

    fn can_insert_more(&self) -> bool {
        self.char_limit
            .is_none_or(|limit| self.total_chars() < limit)
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
            let chars = line_chars(&self.lines[self.cursor_row]);
            let (start, end) = previous_display_cell_char_span(&chars, self.cursor_col);
            let start_off = Self::byte_off(&self.lines[self.cursor_row], start);
            let end_off = Self::byte_off(&self.lines[self.cursor_row], end);
            self.lines[self.cursor_row].replace_range(start_off..end_off, "");
            self.cursor_col = start;
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
            let chars = line_chars(&self.lines[self.cursor_row]);
            let (start, end) = display_cell_char_span(&chars, self.cursor_col);
            let start_off = Self::byte_off(&self.lines[self.cursor_row], start);
            let end_off = Self::byte_off(&self.lines[self.cursor_row], end);
            self.lines[self.cursor_row].replace_range(start_off..end_off, "");
            self.cursor_col = start;
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
            let chars = line_chars(&self.lines[self.cursor_row]);
            self.cursor_col = previous_display_cell_char_span(&chars, self.cursor_col).0;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = Self::char_len(&self.lines[self.cursor_row]);
            self.ensure_visible();
        }
    }

    fn move_right(&mut self) {
        if self.cursor_col < Self::char_len(&self.lines[self.cursor_row]) {
            let chars = line_chars(&self.lines[self.cursor_row]);
            self.cursor_col = display_cell_char_span(&chars, self.cursor_col).1;
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

    fn move_word_left(&mut self) {
        if self.cursor_col > 0 {
            let chars = line_chars(&self.lines[self.cursor_row]);
            self.cursor_col = previous_word_boundary(&chars, self.cursor_col);
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            let chars = line_chars(&self.lines[self.cursor_row]);
            self.cursor_col = previous_word_boundary(&chars, chars.len());
            self.ensure_visible();
        }
    }

    fn move_word_right(&mut self) {
        let line_len = Self::char_len(&self.lines[self.cursor_row]);
        if self.cursor_col < line_len {
            let chars = line_chars(&self.lines[self.cursor_row]);
            self.cursor_col = next_word_boundary(&chars, self.cursor_col);
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            let chars = line_chars(&self.lines[self.cursor_row]);
            self.cursor_col = next_word_boundary(&chars, 0);
            self.ensure_visible();
        }
    }

    fn delete_word_backward(&mut self) -> bool {
        if self.cursor_col > 0 {
            let chars = line_chars(&self.lines[self.cursor_row]);
            let start = previous_word_boundary(&chars, self.cursor_col);
            if start == self.cursor_col {
                return false;
            }
            let start_off = Self::byte_off(&self.lines[self.cursor_row], start);
            let end_off = Self::byte_off(&self.lines[self.cursor_row], self.cursor_col);
            self.lines[self.cursor_row].replace_range(start_off..end_off, "");
            self.cursor_col = start;
            true
        } else if self.cursor_row > 0 {
            self.delete_backward()
        } else {
            false
        }
    }

    fn delete_word_forward(&mut self) -> bool {
        let line_len = Self::char_len(&self.lines[self.cursor_row]);
        if self.cursor_col < line_len {
            let chars = line_chars(&self.lines[self.cursor_row]);
            let end = next_word_boundary(&chars, self.cursor_col);
            if end == self.cursor_col {
                return false;
            }
            let start_off = Self::byte_off(&self.lines[self.cursor_row], self.cursor_col);
            let end_off = Self::byte_off(&self.lines[self.cursor_row], end);
            self.lines[self.cursor_row].replace_range(start_off..end_off, "");
            true
        } else if self.cursor_row + 1 < self.lines.len() {
            self.delete_forward()
        } else {
            false
        }
    }

    fn ensure_visible(&mut self) {
        self.clamp_cursor();
        let h = self.height as usize;
        if h == 0 {
            self.offset = 0;
            return;
        }

        if self.cursor_row < self.offset {
            self.offset = self.cursor_row;
        } else if self.cursor_row >= self.offset.saturating_add(h) {
            self.offset = self.cursor_row.saturating_sub(h).saturating_add(1);
        }
    }

    fn visible_range(&self) -> (usize, usize) {
        let h = self.height as usize;
        if h == 0 || self.lines.is_empty() {
            return (0, 0);
        }

        let (cursor_row, _) = self.normalized_cursor();
        let start = self.normalized_offset_for_cursor(cursor_row);
        let end = start.saturating_add(h).min(self.lines.len());
        (start, end)
    }

    fn normalized_cursor(&self) -> (usize, usize) {
        let cursor_row = self.cursor_row.min(self.lines.len().saturating_sub(1));
        let cursor_col = self
            .lines
            .get(cursor_row)
            .map(|line| self.cursor_col.min(Self::char_len(line)))
            .unwrap_or(0);
        (cursor_row, cursor_col)
    }

    fn clamp_cursor(&mut self) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let (cursor_row, cursor_col) = self.normalized_cursor();
        self.cursor_row = cursor_row;
        self.cursor_col = cursor_col;
    }

    fn normalized_offset(&self) -> usize {
        let h = self.height as usize;
        if h == 0 || self.lines.is_empty() {
            return 0;
        }

        self.offset.min(self.lines.len().saturating_sub(h))
    }

    fn normalized_offset_for_cursor(&self, cursor_row: usize) -> usize {
        let h = self.height as usize;
        if h == 0 || self.lines.is_empty() {
            return 0;
        }

        let max_start = self.lines.len().saturating_sub(h.min(self.lines.len()));
        let mut start = self.normalized_offset();
        if cursor_row < start {
            start = cursor_row;
        } else if cursor_row >= start.saturating_add(h) {
            start = cursor_row.saturating_add(1).saturating_sub(h);
        }
        start.min(max_start)
    }
}

fn split_value_lines(value: &str) -> Vec<String> {
    let mut parts = value.split('\n').peekable();
    let mut lines = Vec::new();

    while let Some(line) = parts.next() {
        let normalized = if parts.peek().is_some() {
            line.strip_suffix('\r').unwrap_or(line)
        } else {
            line
        };
        lines.push(normalized.to_string());
    }

    lines
}

fn clamp_lines_to_char_limit(lines: Vec<String>, limit: usize) -> Vec<String> {
    if limit == 0 {
        return vec![String::new()];
    }

    let line_count = lines.len();
    let mut remaining = limit;
    let mut out = Vec::new();

    for (index, line) in lines.into_iter().enumerate() {
        let line_len = line.chars().count();
        if line_len > remaining {
            out.push(line.chars().take(remaining).collect());
            return out;
        }

        out.push(line);
        remaining -= line_len;

        let has_next_line = index + 1 < line_count;
        if has_next_line {
            if remaining == 0 {
                return out;
            }
            remaining -= 1;
            if remaining == 0 {
                out.push(String::new());
                return out;
            }
        }
    }

    if out.is_empty() {
        vec![String::new()]
    } else {
        out
    }
}

fn line_chars(line: &str) -> Vec<char> {
    line.chars().collect()
}

fn text_char_modifier(modifiers: KeyModifiers) -> bool {
    !modifiers.intersects(
        KeyModifiers::CONTROL
            | KeyModifiers::ALT
            | KeyModifiers::SUPER
            | KeyModifiers::HYPER
            | KeyModifiers::META,
    )
}

fn word_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

fn previous_word_boundary(chars: &[char], cursor: usize) -> usize {
    let mut index = cursor.min(chars.len());
    while index > 0 && chars[index - 1].is_whitespace() {
        index -= 1;
    }
    while index > 0 && !chars[index - 1].is_whitespace() {
        index -= 1;
    }
    index
}

fn next_word_boundary(chars: &[char], cursor: usize) -> usize {
    let mut index = cursor.min(chars.len());
    while index < chars.len() && !chars[index].is_whitespace() {
        index += 1;
    }
    while index < chars.len() && chars[index].is_whitespace() {
        index += 1;
    }
    index
}

impl Default for Textarea {
    fn default() -> Self {
        Self::new()
    }
}

impl Textarea {
    pub fn element<Msg>(&self) -> Element<Msg> {
        if self.height == 0 || self.width == 0 {
            return Element::Box(BoxElement::new().direction(FlexDirection::Column));
        }

        if self.lines == vec![String::new()] && !self.placeholder.is_empty() && !self.focused {
            return Element::Text(
                TextElement::new(&self.placeholder)
                    .dim()
                    .fg(Color::BrightBlack),
            );
        }

        let h = self.height as usize;
        let (cursor_row, _) = self.normalized_cursor();
        let (start, end) = self.visible_range();
        let visible = &self.lines[start..end];

        let mut children: Vec<Element<Msg>> = Vec::new();
        for (i, line) in visible.iter().enumerate() {
            let row = start + i;
            if row == cursor_row && self.focused {
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
#[path = "textarea_tests.rs"]
mod tests;
