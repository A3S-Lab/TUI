use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{Event, KeyEvent};
use crate::style::{display_cell_char_span, previous_display_cell_char_span, Color};
use crossterm::event::{KeyCode, KeyModifiers};

pub struct TextInput {
    value: String,
    /// Cursor position as a char index. Convert to a byte offset before String edits.
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

    /// Current cursor position as a char index.
    pub fn cursor(&self) -> usize {
        self.normalized_cursor()
    }

    pub fn set_value(&mut self, v: impl Into<String>) {
        let value = v.into();
        self.value = match self.char_limit {
            Some(limit) => value.chars().take(limit).collect(),
            None => value,
        };
        self.cursor = Self::char_len(&self.value);
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<TextInputMsg> {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Paste(text) => self.handle_paste(text),
            _ => None,
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<TextInputMsg> {
        if !self.focused {
            return None;
        }
        self.clamp_cursor();
        match (key.code, key.modifiers) {
            (KeyCode::Left, modifiers) if word_modifier(modifiers) => {
                self.move_word_left();
                None
            }
            (KeyCode::Right, modifiers) if word_modifier(modifiers) => {
                self.move_word_right();
                None
            }
            (KeyCode::Char('b'), modifiers) if modifiers.contains(KeyModifiers::ALT) => {
                self.move_word_left();
                None
            }
            (KeyCode::Char('f'), modifiers) if modifiers.contains(KeyModifiers::ALT) => {
                self.move_word_right();
                None
            }
            (KeyCode::Char('w'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                if self.delete_word_backward() {
                    Some(TextInputMsg::Changed(self.value.clone()))
                } else {
                    None
                }
            }
            (KeyCode::Backspace, modifiers) if word_modifier(modifiers) => {
                if self.delete_word_backward() {
                    Some(TextInputMsg::Changed(self.value.clone()))
                } else {
                    None
                }
            }
            (KeyCode::Delete, modifiers) if word_modifier(modifiers) => {
                if self.delete_word_forward() {
                    Some(TextInputMsg::Changed(self.value.clone()))
                } else {
                    None
                }
            }
            (KeyCode::Char('d'), modifiers) if modifiers.contains(KeyModifiers::ALT) => {
                if self.delete_word_forward() {
                    Some(TextInputMsg::Changed(self.value.clone()))
                } else {
                    None
                }
            }
            (KeyCode::Char(c), modifiers) if text_char_modifier(modifiers) => {
                if self.insert_char(c) {
                    Some(TextInputMsg::Changed(self.value.clone()))
                } else {
                    None
                }
            }
            (KeyCode::Backspace, _) => {
                if self.cursor > 0 {
                    let chars = self.value_chars();
                    let (start, end) = previous_display_cell_char_span(&chars, self.cursor);
                    let start_offset = Self::byte_off(&self.value, start);
                    let end_offset = Self::byte_off(&self.value, end);
                    self.value.replace_range(start_offset..end_offset, "");
                    self.cursor = start;
                    Some(TextInputMsg::Changed(self.value.clone()))
                } else {
                    None
                }
            }
            (KeyCode::Delete, _) => {
                if self.cursor < Self::char_len(&self.value) {
                    let chars = self.value_chars();
                    let (start, end) = display_cell_char_span(&chars, self.cursor);
                    let start_offset = Self::byte_off(&self.value, start);
                    let end_offset = Self::byte_off(&self.value, end);
                    self.value.replace_range(start_offset..end_offset, "");
                    self.cursor = start;
                    Some(TextInputMsg::Changed(self.value.clone()))
                } else {
                    None
                }
            }
            (KeyCode::Left, _) => {
                let chars = self.value_chars();
                self.cursor = previous_display_cell_char_span(&chars, self.cursor).0;
                None
            }
            (KeyCode::Right, _) => {
                let chars = self.value_chars();
                self.cursor = display_cell_char_span(&chars, self.cursor).1;
                None
            }
            (KeyCode::Home, _) => {
                self.cursor = 0;
                None
            }
            (KeyCode::End, _) => {
                self.cursor = Self::char_len(&self.value);
                None
            }
            (KeyCode::Enter, _) => Some(TextInputMsg::Submit(self.value.clone())),
            _ => None,
        }
    }

    /// Insert pasted text at the cursor. Newlines and tabs are converted to
    /// spaces, carriage returns and other control characters are dropped.
    pub fn insert_str(&mut self, text: &str) -> bool {
        self.clamp_cursor();
        let mut changed = false;
        for ch in sanitize_single_line_paste(text).chars() {
            if !self.insert_char(ch) {
                break;
            }
            changed = true;
        }
        changed
    }

    fn handle_paste(&mut self, text: &str) -> Option<TextInputMsg> {
        if !self.focused {
            return None;
        }
        if self.insert_str(text) {
            Some(TextInputMsg::Changed(self.value.clone()))
        } else {
            None
        }
    }

    fn insert_char(&mut self, c: char) -> bool {
        if !self.can_insert_more() {
            return false;
        }
        let offset = Self::byte_off(&self.value, self.cursor);
        self.value.insert(offset, c);
        self.cursor += 1;
        true
    }

    fn can_insert_more(&self) -> bool {
        self.char_limit
            .is_none_or(|limit| Self::char_len(&self.value) < limit)
    }

    fn move_word_left(&mut self) {
        let chars = self.value_chars();
        self.cursor = previous_word_boundary(&chars, self.cursor);
    }

    fn move_word_right(&mut self) {
        let chars = self.value_chars();
        self.cursor = next_word_boundary(&chars, self.cursor);
    }

    fn delete_word_backward(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }

        let chars = self.value_chars();
        let start = previous_word_boundary(&chars, self.cursor);
        if start == self.cursor {
            return false;
        }

        let start_offset = Self::byte_off(&self.value, start);
        let end_offset = Self::byte_off(&self.value, self.cursor);
        self.value.replace_range(start_offset..end_offset, "");
        self.cursor = start;
        true
    }

    fn delete_word_forward(&mut self) -> bool {
        let len = Self::char_len(&self.value);
        if self.cursor >= len {
            return false;
        }

        let chars = self.value_chars();
        let end = next_word_boundary(&chars, self.cursor);
        if end == self.cursor {
            return false;
        }

        let start_offset = Self::byte_off(&self.value, self.cursor);
        let end_offset = Self::byte_off(&self.value, end);
        self.value.replace_range(start_offset..end_offset, "");
        true
    }

    fn byte_off(value: &str, col: usize) -> usize {
        value
            .char_indices()
            .nth(col)
            .map_or(value.len(), |(b, _)| b)
    }

    fn char_len(value: &str) -> usize {
        value.chars().count()
    }

    fn display_chars(&self) -> Vec<char> {
        if let Some(mask) = self.mask_char {
            vec![mask; Self::char_len(&self.value)]
        } else {
            self.value_chars()
        }
    }

    fn value_chars(&self) -> Vec<char> {
        self.value.chars().collect()
    }

    fn normalized_cursor(&self) -> usize {
        self.cursor.min(Self::char_len(&self.value))
    }

    fn clamp_cursor(&mut self) {
        self.cursor = self.normalized_cursor();
    }

    fn placeholder_cursor_parts(&self) -> (String, String) {
        let chars = self.placeholder.chars().collect::<Vec<_>>();
        let (start, end) = display_cell_char_span(&chars, 0);
        let cursor = chars[start..end].iter().collect::<String>();
        let after = chars[end..].iter().collect::<String>();
        (cursor, after)
    }

    pub fn view(&self) -> String {
        let mut out = self.prefix.clone();

        if self.value.is_empty() && !self.placeholder.is_empty() {
            if self.focused {
                let (cursor, after) = self.placeholder_cursor_parts();
                out.push_str(&format!("\x1b[2;7m{cursor}\x1b[0m"));
                if !after.is_empty() {
                    out.push_str(&format!("\x1b[2m{after}\x1b[0m"));
                }
            } else {
                out.push_str(&format!("\x1b[2m{}\x1b[0m", self.placeholder));
            }
            return out;
        }

        let display_chars = self.display_chars();
        let cursor = self.normalized_cursor().min(display_chars.len());
        let (cursor_start, cursor_end) = display_cell_char_span(&display_chars, cursor);

        for (i, &ch) in display_chars.iter().enumerate() {
            if self.focused && i == cursor_start && cursor_start < display_chars.len() {
                let cursor_text = display_chars[cursor_start..cursor_end]
                    .iter()
                    .collect::<String>();
                out.push_str(&format!("\x1b[7m{}\x1b[0m", cursor_text));
            } else if !(self.focused && i > cursor_start && i < cursor_end) {
                out.push(ch);
            }
        }
        if cursor == display_chars.len() && self.focused {
            out.push_str("\x1b[7m \x1b[0m");
        }
        out
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        if self.value.is_empty() && !self.placeholder.is_empty() {
            if !self.focused {
                let text = format!("{}{}", self.prefix, self.placeholder);
                return Element::Text(TextElement::new(text).dim().fg(Color::BrightBlack));
            }

            let mut children = Vec::new();
            if !self.prefix.is_empty() {
                children.push(Element::Text(TextElement::new(self.prefix.clone())));
            }
            let (cursor, after) = self.placeholder_cursor_parts();
            children.push(Element::Text(
                TextElement::new(cursor)
                    .dim()
                    .fg(Color::BrightBlack)
                    .reverse(),
            ));
            if !after.is_empty() {
                children.push(Element::Text(
                    TextElement::new(after).dim().fg(Color::BrightBlack),
                ));
            }
            return Element::Box(
                BoxElement::new()
                    .direction(FlexDirection::Row)
                    .children(children),
            );
        }

        let display_chars = self.display_chars();
        let cursor = self.normalized_cursor().min(display_chars.len());

        let mut children = Vec::new();
        if !self.prefix.is_empty() {
            children.push(Element::Text(TextElement::new(self.prefix.clone())));
        }

        let (cursor_start, cursor_end) = display_cell_char_span(&display_chars, cursor);
        let before = display_chars.iter().take(cursor_start).collect::<String>();
        if !before.is_empty() {
            children.push(Element::Text(TextElement::new(before)));
        }

        if self.focused {
            let cursor_text = if cursor_start < display_chars.len() {
                display_chars[cursor_start..cursor_end]
                    .iter()
                    .collect::<String>()
            } else {
                " ".to_string()
            };
            children.push(Element::Text(TextElement::new(cursor_text).reverse()));
            let after = display_chars.iter().skip(cursor_end).collect::<String>();
            if !after.is_empty() {
                children.push(Element::Text(TextElement::new(after)));
            }
        } else {
            let value = display_chars.iter().skip(cursor).collect::<String>();
            if !value.is_empty() {
                children.push(Element::Text(TextElement::new(value)));
            }
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }
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

fn sanitize_single_line_paste(text: &str) -> String {
    text.chars()
        .filter_map(|ch| match ch {
            '\r' => None,
            '\n' | '\t' => Some(' '),
            ch if ch.is_control() => None,
            ch => Some(ch),
        })
        .collect()
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

    fn alt(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::ALT,
        }
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
    fn handle_event_paste_sanitizes_single_line_text() {
        let mut input = TextInput::new();

        let msg = input.handle_event(&Event::Paste("hello\r\nworld\t!\u{7}".to_string()));

        assert!(matches!(msg, Some(TextInputMsg::Changed(value)) if value == "hello world !"));
        assert_eq!(input.value(), "hello world !");
    }

    #[test]
    fn paste_respects_char_limit_at_cursor() {
        let mut input = TextInput::new().with_char_limit(4);
        input.set_value("ab");
        input.handle_key(&key(KeyCode::Home));
        input.handle_key(&key(KeyCode::Right));

        input.insert_str("XYZ");

        assert_eq!(input.value(), "aXYb");
        assert_eq!(input.cursor(), 3);
    }

    #[test]
    fn control_modified_characters_are_not_text_input() {
        let mut input = TextInput::new();

        assert!(input.handle_key(&ctrl(KeyCode::Char('c'))).is_none());

        assert_eq!(input.value(), "");
    }

    #[test]
    fn word_navigation_and_deletion() {
        let mut input = TextInput::new();
        input.set_value("hello brave world");

        input.handle_key(&alt(KeyCode::Char('b')));
        assert_eq!(input.cursor(), 12);

        input.handle_key(&alt(KeyCode::Char('d')));
        assert_eq!(input.value(), "hello brave ");
        assert_eq!(input.cursor(), 12);

        input.handle_key(&ctrl(KeyCode::Char('w')));
        assert_eq!(input.value(), "hello ");
        assert_eq!(input.cursor(), 6);

        input.handle_key(&key(KeyCode::Home));
        input.handle_key(&alt(KeyCode::Char('d')));
        assert_eq!(input.value(), "");
        assert_eq!(input.cursor(), 0);
    }

    #[test]
    fn char_limit_counts_multibyte_chars() {
        let mut input = TextInput::new().with_char_limit(2);
        input.handle_key(&key(KeyCode::Char('你')));
        input.handle_key(&key(KeyCode::Char('好')));
        input.handle_key(&key(KeyCode::Char('a')));

        assert_eq!(input.value(), "你好");
    }

    #[test]
    fn set_value_honors_char_limit_on_char_boundaries() {
        let mut input = TextInput::new().with_char_limit(2);
        input.set_value("你好abc");

        assert_eq!(input.value(), "你好");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn set_value_honors_zero_char_limit() {
        let mut input = TextInput::new().with_char_limit(0);
        input.set_value("hello");

        assert_eq!(input.value(), "");
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn multibyte_input_edits_on_char_boundaries() {
        let mut input = TextInput::new();
        for ch in "你好abc".chars() {
            input.handle_key(&key(KeyCode::Char(ch)));
        }

        for _ in 0..3 {
            input.handle_key(&key(KeyCode::Left));
        }
        input.handle_key(&key(KeyCode::Backspace));
        input.handle_key(&key(KeyCode::Delete));

        assert_eq!(input.value(), "你bc");
    }

    #[test]
    fn multibyte_end_cursor_renders_after_value() {
        let mut input = TextInput::new();
        input.set_value("你好");

        assert!(input.view().ends_with("\x1b[7m \x1b[0m"));

        let Element::Box(row) = input.element::<()>() else {
            panic!("expected row element");
        };
        let Element::Text(cursor) = row.children.last().expect("expected cursor child") else {
            panic!("expected cursor text");
        };
        assert_eq!(cursor.content, " ");
        assert!(cursor.style.reverse);
    }

    #[test]
    fn element_uses_structured_cursor_style() {
        let mut input = TextInput::new();
        input.set_value("abc");
        input.handle_key(&key(KeyCode::Home));
        input.handle_key(&key(KeyCode::Right));

        let Element::Box(row) = input.element::<()>() else {
            panic!("expected row element");
        };
        assert_eq!(row.children.len(), 3);
        let Element::Text(cursor) = &row.children[1] else {
            panic!("expected cursor text");
        };
        assert_eq!(cursor.content, "b");
        assert!(cursor.style.reverse);
        assert!(!cursor.content.contains('\x1b'));
    }

    #[test]
    fn focused_placeholder_keeps_cursor_visible() {
        let input = TextInput::new()
            .with_prefix("> ")
            .with_placeholder("command");

        let view = input.view();

        assert_eq!(crate::style::strip_ansi(&view), "> command");
        assert!(view.contains("\x1b[2;7mc\x1b[0m"));

        let Element::Box(row) = input.element::<()>() else {
            panic!("expected row element");
        };
        assert_eq!(row.children.len(), 3);
        let Element::Text(cursor) = &row.children[1] else {
            panic!("expected cursor text");
        };
        assert_eq!(cursor.content, "c");
        assert!(cursor.style.reverse);
        assert!(cursor.style.dim);
    }

    #[test]
    fn cursor_styles_following_zero_width_marks_with_base_glyph() {
        let mut input = TextInput::new();
        input.set_value("e\u{301}x");
        input.handle_key(&key(KeyCode::Home));

        assert_eq!(crate::style::strip_ansi(&input.view()), "e\u{301}x");
        assert!(input.view().contains("\x1b[7me\u{301}\x1b[0mx"));

        let Element::Box(row) = input.element::<()>() else {
            panic!("expected row element");
        };
        let Element::Text(cursor) = &row.children[0] else {
            panic!("expected cursor text");
        };
        assert_eq!(cursor.content, "e\u{301}");
        assert!(cursor.style.reverse);
    }

    #[test]
    fn cursor_inside_zero_width_span_styles_base_glyph() {
        let mut input = TextInput::new();
        input.set_value("e\u{301}x");
        input.cursor = 1;

        assert_eq!(input.cursor, 1);
        assert_eq!(crate::style::strip_ansi(&input.view()), "e\u{301}x");
        assert!(input.view().contains("\x1b[7me\u{301}\x1b[0mx"));

        let Element::Box(row) = input.element::<()>() else {
            panic!("expected row element");
        };
        let Element::Text(cursor) = &row.children[0] else {
            panic!("expected cursor text");
        };
        assert_eq!(cursor.content, "e\u{301}");
        assert!(cursor.style.reverse);
    }

    #[test]
    fn cursor_movement_skips_zero_width_marks() {
        let mut input = TextInput::new();
        input.set_value("e\u{301}x");
        input.handle_key(&key(KeyCode::Home));

        input.handle_key(&key(KeyCode::Right));
        assert_eq!(input.cursor, 2);
        assert!(input.view().contains("e\u{301}\x1b[7mx\x1b[0m"));

        input.handle_key(&key(KeyCode::Right));
        assert_eq!(input.cursor, 3);

        input.handle_key(&key(KeyCode::Left));
        assert_eq!(input.cursor, 2);
        input.handle_key(&key(KeyCode::Left));
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn edit_keys_remove_zero_width_span_with_base_glyph() {
        let mut input = TextInput::new();
        input.set_value("e\u{301}x");
        input.handle_key(&key(KeyCode::Home));
        input.handle_key(&key(KeyCode::Delete));

        assert_eq!(input.value(), "x");
        assert_eq!(input.cursor, 0);

        input.set_value("e\u{301}x");
        input.handle_key(&key(KeyCode::Home));
        input.handle_key(&key(KeyCode::Right));
        input.handle_key(&key(KeyCode::Backspace));

        assert_eq!(input.value(), "x");
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn editing_normalizes_stale_cursor() {
        let mut input = TextInput::new();
        input.set_value("ab");
        input.cursor = usize::MAX;

        input.handle_key(&key(KeyCode::Char('!')));

        assert_eq!(input.value(), "ab!");
        assert_eq!(input.cursor, 3);
    }

    #[test]
    fn rendering_normalizes_stale_cursor() {
        let mut input = TextInput::new();
        input.set_value("ab");
        input.cursor = usize::MAX;

        assert!(input.view().ends_with("\x1b[7m \x1b[0m"));

        let Element::Box(row) = input.element::<()>() else {
            panic!("expected row element");
        };
        let Element::Text(cursor) = row.children.last().expect("expected cursor child") else {
            panic!("expected cursor text");
        };
        assert_eq!(cursor.content, " ");
        assert!(cursor.style.reverse);
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

        let mut masked = TextInput::new().with_mask('*');
        masked.set_value("你好");
        masked.blur();
        assert_eq!(masked.view(), "**");
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
