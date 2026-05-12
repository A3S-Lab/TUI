use crate::event::KeyEvent;
use crossterm::event::KeyCode;

pub struct TextInput {
    value: String,
    cursor: usize,
    placeholder: String,
    focused: bool,
    char_limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum TextInputMsg {
    Changed(String),
    Submit(String),
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            placeholder: String::new(),
            focused: true,
            char_limit: None,
        }
    }

    pub fn with_placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }

    pub fn with_char_limit(mut self, limit: usize) -> Self {
        self.char_limit = Some(limit);
        self
    }

    pub fn focus(&mut self) {
        self.focused = true;
    }

    pub fn blur(&mut self) {
        self.focused = false;
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, v: impl Into<String>) {
        self.value = v.into();
        self.cursor = self.value.len();
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<TextInputMsg> {
        if !self.focused {
            return None;
        }
        match key.code {
            KeyCode::Char(c) => {
                if let Some(limit) = self.char_limit {
                    if self.value.len() >= limit {
                        return None;
                    }
                }
                self.value.insert(self.cursor, c);
                self.cursor += 1;
                Some(TextInputMsg::Changed(self.value.clone()))
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.value.remove(self.cursor);
                    Some(TextInputMsg::Changed(self.value.clone()))
                } else {
                    None
                }
            }
            KeyCode::Delete => {
                if self.cursor < self.value.len() {
                    self.value.remove(self.cursor);
                    Some(TextInputMsg::Changed(self.value.clone()))
                } else {
                    None
                }
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                None
            }
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.value.len());
                None
            }
            KeyCode::Home => {
                self.cursor = 0;
                None
            }
            KeyCode::End => {
                self.cursor = self.value.len();
                None
            }
            KeyCode::Enter => Some(TextInputMsg::Submit(self.value.clone())),
            _ => None,
        }
    }

    pub fn view(&self) -> String {
        if self.value.is_empty() && !self.placeholder.is_empty() {
            return format!("\x1b[2m{}\x1b[0m", self.placeholder);
        }

        let mut out = String::new();
        for (i, ch) in self.value.chars().enumerate() {
            if i == self.cursor && self.focused {
                out.push_str(&format!("\x1b[7m{}\x1b[0m", ch));
            } else {
                out.push(ch);
            }
        }
        if self.cursor == self.value.len() && self.focused {
            out.push_str("\x1b[7m \x1b[0m");
        }
        out
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}
