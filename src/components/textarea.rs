use crate::event::KeyEvent;
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

    pub fn focus(&mut self) { self.focused = true; }
    pub fn blur(&mut self) { self.focused = false; }

    pub fn value(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_value(&mut self, v: &str) {
        self.lines = v.lines().map(|l| l.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = self.lines.len() - 1;
        self.cursor_col = self.lines[self.cursor_row].len();
    }

    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.offset = 0;
    }

    pub fn total_chars(&self) -> usize {
        self.lines.iter().map(|l| l.len()).sum::<usize>() + self.lines.len() - 1
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<TextareaMsg> {
        if !self.focused {
            return None;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Enter, KeyModifiers::NONE) if self.submit_on_enter => {
                Some(TextareaMsg::Submit(self.value()))
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
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
                self.cursor_col = self.lines[self.cursor_row].len();
                None
            }
            (KeyCode::Char('k'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.lines[self.cursor_row].truncate(self.cursor_col);
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
            (KeyCode::Left, _) => { self.move_left(); None }
            (KeyCode::Right, _) => { self.move_right(); None }
            (KeyCode::Up, _) => { self.move_up(); None }
            (KeyCode::Down, _) => { self.move_down(); None }
            (KeyCode::Home, _) => { self.cursor_col = 0; None }
            (KeyCode::End, _) => {
                self.cursor_col = self.lines[self.cursor_row].len();
                None
            }
            _ => None,
        }
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

    fn render_line_with_cursor(&self, line: &str) -> String {
        let mut out = String::new();
        let chars: Vec<char> = line.chars().collect();

        for (i, &ch) in chars.iter().enumerate() {
            if i == self.cursor_col {
                out.push_str(&format!("\x1b[7m{}\x1b[0m", ch));
            } else {
                out.push(ch);
            }
        }

        if self.cursor_col >= chars.len() {
            out.push_str("\x1b[7m \x1b[0m");
        }

        out
    }

    fn insert_char(&mut self, c: char) {
        self.lines[self.cursor_row].insert(self.cursor_col, c);
        self.cursor_col += 1;
    }

    fn insert_newline(&mut self) {
        let rest = self.lines[self.cursor_row].split_off(self.cursor_col);
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;
        self.ensure_visible();
    }

    fn delete_backward(&mut self) -> bool {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            self.lines[self.cursor_row].remove(self.cursor_col);
            true
        } else if self.cursor_row > 0 {
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
            self.ensure_visible();
            true
        } else {
            false
        }
    }

    fn delete_forward(&mut self) -> bool {
        if self.cursor_col < self.lines[self.cursor_row].len() {
            self.lines[self.cursor_row].remove(self.cursor_col);
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
            self.cursor_col = self.lines[self.cursor_row].len();
            self.ensure_visible();
        }
    }

    fn move_right(&mut self) {
        if self.cursor_col < self.lines[self.cursor_row].len() {
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
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
            self.ensure_visible();
        }
    }

    fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
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
