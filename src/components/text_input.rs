use crate::element::{Element, TextElement};
use crate::event::KeyEvent;
use crate::style::Color;
use crossterm::event::KeyCode;

pub struct TextInput {
    value: String,
    cursor: usize,
    placeholder: String,
    focused: bool,
    char_limit: Option<usize>,
    mask_char: Option<char>,
    prefix: String,
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
            mask_char: None,
            prefix: String::new(),
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

    /// Enable password mode — display a mask character instead of actual input.
    pub fn with_mask(mut self, ch: char) -> Self {
        self.mask_char = Some(ch);
        self
    }

    /// Set a prefix displayed before the input (e.g., "> " or "$ ").
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
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
        let mut out = self.prefix.clone();

        if self.value.is_empty() && !self.placeholder.is_empty() {
            out.push_str(&format!("\x1b[2m{}\x1b[0m", self.placeholder));
            return out;
        }

        let display_chars: Vec<char> = if let Some(mask) = self.mask_char {
            vec![mask; self.value.len()]
        } else {
            self.value.chars().collect()
        };

        for (i, &ch) in display_chars.iter().enumerate() {
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

    pub fn element<Msg>(&self) -> Element<Msg> {
        if self.value.is_empty() && !self.placeholder.is_empty() {
            let text = format!("{}{}", self.prefix, self.placeholder);
            return Element::Text(
                TextElement::new(text).dim().fg(Color::BrightBlack),
            );
        }

        let display_chars: Vec<char> = if let Some(mask) = self.mask_char {
            vec![mask; self.value.len()]
        } else {
            self.value.chars().collect()
        };

        let mut display = self.prefix.clone();
        for (i, &ch) in display_chars.iter().enumerate() {
            if i == self.cursor && self.focused {
                display.push_str(&format!("\x1b[7m{}\x1b[0m", ch));
            } else {
                display.push(ch);
            }
        }
        if self.cursor == self.value.len() && self.focused {
            display.push_str("\x1b[7m \x1b[0m");
        }

        Element::Text(TextElement::new(display))
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent { code, modifiers: KeyModifiers::NONE }
    }

    #[test]
    fn typing_characters() {
        let mut input = TextInput::new();
        input.handle_key(&key(KeyCode::Char('h')));
        input.handle_key(&key(KeyCode::Char('i')));
        assert_eq!(input.value(), "hi");
    }

    #[test]
    fn backspace_deletes() {
        let mut input = TextInput::new();
        input.handle_key(&key(KeyCode::Char('a')));
        input.handle_key(&key(KeyCode::Char('b')));
        input.handle_key(&key(KeyCode::Backspace));
        assert_eq!(input.value(), "a");
    }

    #[test]
    fn cursor_movement() {
        let mut input = TextInput::new();
        input.set_value("hello");
        input.handle_key(&key(KeyCode::Home));
        assert_eq!(input.cursor, 0);
        input.handle_key(&key(KeyCode::End));
        assert_eq!(input.cursor, 5);
        input.handle_key(&key(KeyCode::Left));
        assert_eq!(input.cursor, 4);
        input.handle_key(&key(KeyCode::Right));
        assert_eq!(input.cursor, 5);
    }

    #[test]
    fn char_limit() {
        let mut input = TextInput::new().with_char_limit(3);
        input.handle_key(&key(KeyCode::Char('a')));
        input.handle_key(&key(KeyCode::Char('b')));
        input.handle_key(&key(KeyCode::Char('c')));
        input.handle_key(&key(KeyCode::Char('d')));
        assert_eq!(input.value(), "abc");
    }

    #[test]
    fn submit_returns_value() {
        let mut input = TextInput::new();
        input.set_value("test");
        let msg = input.handle_key(&key(KeyCode::Enter));
        assert!(matches!(msg, Some(TextInputMsg::Submit(s)) if s == "test"));
    }

    #[test]
    fn blur_ignores_input() {
        let mut input = TextInput::new();
        input.blur();
        input.handle_key(&key(KeyCode::Char('x')));
        assert_eq!(input.value(), "");
    }

    #[test]
    fn delete_key() {
        let mut input = TextInput::new();
        input.set_value("abc");
        input.handle_key(&key(KeyCode::Home));
        input.handle_key(&key(KeyCode::Delete));
        assert_eq!(input.value(), "bc");
    }

    #[test]
    fn mask_mode() {
        let input = TextInput::new().with_mask('*');
        assert_eq!(input.mask_char, Some('*'));
    }

    #[test]
    fn prefix_in_view() {
        let mut input = TextInput::new().with_prefix("> ");
        input.set_value("hello");
        input.blur();
        let view = input.view();
        assert!(view.starts_with("> "));
    }
}
