use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseEvent, MouseEventKind};
use crate::style::{truncate_visible, visible_len, Color, Style};
use crossterm::event::KeyCode;

pub struct Select {
    items: Vec<String>,
    cursor: usize,
    focused: bool,
    y_offset: u16,
    number_shortcuts: bool,
}

#[derive(Debug, Clone)]
pub enum SelectMsg {
    Selected(usize, String),
}

impl Select {
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(|i| i.into()).collect(),
            cursor: 0,
            focused: true,
            y_offset: 0,
            number_shortcuts: false,
        }
    }

    pub fn with_selected(mut self, selected: usize) -> Self {
        self.cursor = selected.min(self.items.len().saturating_sub(1));
        self
    }

    pub fn with_number_shortcuts(mut self) -> Self {
        self.number_shortcuts = true;
        self
    }

    pub fn focus(&mut self) {
        self.focused = true;
    }
    pub fn blur(&mut self) {
        self.focused = false;
    }
    pub fn selected_index(&self) -> usize {
        self.cursor
    }
    pub fn selected_value(&self) -> &str {
        &self.items[self.cursor]
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Set the vertical offset for mouse click calculations.
    pub fn set_y_offset(&mut self, y: u16) {
        self.y_offset = y;
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<SelectMsg> {
        if !self.focused {
            return None;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = self.cursor.saturating_sub(1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor + 1 < self.items.len() {
                    self.cursor += 1;
                }
                None
            }
            KeyCode::Enter => self.selected_msg(),
            KeyCode::Char(c) if self.number_shortcuts => {
                let idx = number_shortcut_index(c)?;
                if idx < self.items.len() {
                    self.cursor = idx;
                    self.selected_msg()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Handle mouse events (click to select).
    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<SelectMsg> {
        if !self.focused {
            return None;
        }
        match mouse.kind {
            MouseEventKind::Down(crate::event::MouseButton::Left) => {
                let row = mouse.row.saturating_sub(self.y_offset) as usize;
                if row < self.items.len() {
                    self.cursor = row;
                    Some(SelectMsg::Selected(
                        self.cursor,
                        self.items[self.cursor].clone(),
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        let start = if self.items.len() <= height {
            0
        } else {
            self.cursor
                .saturating_sub(height - 1)
                .min(self.items.len() - height)
        };

        self.items
            .iter()
            .enumerate()
            .skip(start)
            .take(height)
            .map(|(idx, item)| {
                let prefix = if idx == self.cursor { ">" } else { " " };
                let raw = if self.number_shortcuts {
                    match number_shortcut_label(idx) {
                        Some(label) => pad_or_truncate(&format!("{prefix} {label} {item}"), width),
                        None => pad_or_truncate(&format!("{prefix}   {item}"), width),
                    }
                } else {
                    pad_or_truncate(&format!("{prefix} {item}"), width)
                };
                if idx == self.cursor && self.focused {
                    Style::new().fg(Color::Cyan).bold().render(&raw)
                } else {
                    raw
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let children: Vec<Element<Msg>> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let prefix = if i == self.cursor { "▸ " } else { "  " };
                let text = if self.number_shortcuts {
                    match number_shortcut_label(i) {
                        Some(label) => format!("{prefix}{label} {item}"),
                        None => format!("{prefix}  {item}"),
                    }
                } else {
                    format!("{}{}", prefix, item)
                };
                if i == self.cursor && self.focused {
                    Element::Text(TextElement::new(text).bold().fg(Color::Cyan))
                } else {
                    Element::Text(TextElement::new(text))
                }
            })
            .collect();

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn selected_msg(&self) -> Option<SelectMsg> {
        self.items
            .get(self.cursor)
            .cloned()
            .map(|item| SelectMsg::Selected(self.cursor, item))
    }
}

fn number_shortcut_index(c: char) -> Option<usize> {
    match c {
        '1'..='9' => Some((c as u8 - b'1') as usize),
        '0' => Some(9),
        _ => None,
    }
}

fn number_shortcut_label(idx: usize) -> Option<char> {
    match idx {
        0..=8 => Some((b'1' + idx as u8) as char),
        9 => Some('0'),
        _ => None,
    }
}

fn pad_or_truncate(value: &str, width: usize) -> String {
    let truncated = truncate_visible(value, width);
    let len = visible_len(&truncated);
    if len >= width {
        truncated
    } else {
        format!("{truncated}{}", " ".repeat(width - len))
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

    #[test]
    fn initial_state() {
        let select = Select::new(vec!["a", "b", "c"]);
        assert_eq!(select.selected_index(), 0);
        assert_eq!(select.selected_value(), "a");
    }

    #[test]
    fn initializes_selected_state() {
        let select = Select::new(vec!["a", "b", "c"]).with_selected(9);

        assert_eq!(select.selected_index(), 2);
        assert_eq!(select.cursor(), 2);
    }

    #[test]
    fn navigate_down() {
        let mut select = Select::new(vec!["a", "b", "c"]);
        select.handle_key(&key(KeyCode::Down));
        assert_eq!(select.selected_index(), 1);
        select.handle_key(&key(KeyCode::Char('j')));
        assert_eq!(select.selected_index(), 2);
    }

    #[test]
    fn navigate_up() {
        let mut select = Select::new(vec!["a", "b", "c"]);
        select.handle_key(&key(KeyCode::Down));
        select.handle_key(&key(KeyCode::Down));
        select.handle_key(&key(KeyCode::Up));
        assert_eq!(select.selected_index(), 1);
    }

    #[test]
    fn bounds_check() {
        let mut select = Select::new(vec!["a", "b"]);
        select.handle_key(&key(KeyCode::Up));
        assert_eq!(select.selected_index(), 0);
        select.handle_key(&key(KeyCode::Down));
        select.handle_key(&key(KeyCode::Down));
        assert_eq!(select.selected_index(), 1);
    }

    #[test]
    fn enter_selects() {
        let mut select = Select::new(vec!["x", "y"]);
        select.handle_key(&key(KeyCode::Down));
        let msg = select.handle_key(&key(KeyCode::Enter));
        assert!(matches!(msg, Some(SelectMsg::Selected(1, _))));
    }

    #[test]
    fn number_shortcuts_are_opt_in() {
        let mut select = Select::new(vec!["one", "two", "three"]);

        let msg = select.handle_key(&key(KeyCode::Char('2')));

        assert!(msg.is_none());
        assert_eq!(select.selected_index(), 0);
    }

    #[test]
    fn number_shortcuts_select_by_visible_index() {
        let mut select = Select::new(vec![
            "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        ])
        .with_number_shortcuts();

        let msg = select.handle_key(&key(KeyCode::Char('3')));
        assert!(matches!(msg, Some(SelectMsg::Selected(2, _))));
        assert_eq!(select.selected_index(), 2);

        let msg = select.handle_key(&key(KeyCode::Char('0')));
        assert!(matches!(msg, Some(SelectMsg::Selected(9, _))));
        assert_eq!(select.selected_value(), "ten");
    }

    #[test]
    fn blur_ignores_input() {
        let mut select = Select::new(vec!["a", "b"]);
        select.blur();
        select.handle_key(&key(KeyCode::Down));
        assert_eq!(select.selected_index(), 0);
    }

    #[test]
    fn view_renders_cursor_and_truncates() {
        let select = Select::new(vec!["a very long option", "b"]).with_selected(1);
        let plain = crate::style::strip_ansi(&select.view(10, 5));

        assert!(plain.contains("> b"));
        for line in plain.lines() {
            assert!(visible_len(line) <= 10, "{line:?}");
        }
    }

    #[test]
    fn view_renders_number_shortcuts_when_enabled() {
        let select = Select::new(vec!["cpu", "memory", "network"]).with_number_shortcuts();
        let plain = crate::style::strip_ansi(&select.view(20, 5));

        assert!(plain.contains("> 1 cpu"));
        assert!(plain.contains("  2 memory"));
        assert!(plain.contains("  3 network"));
    }

    #[test]
    fn view_scrolls_to_cursor() {
        let select = Select::new(vec!["one", "two", "three", "four"]).with_selected(3);
        let plain = crate::style::strip_ansi(&select.view(16, 2));

        assert!(!plain.contains("one"));
        assert!(plain.contains("> four"));
    }
}
