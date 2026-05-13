use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseEvent, MouseEventKind};
use crate::style::Color;
use crossterm::event::KeyCode;

pub struct Select {
    items: Vec<String>,
    cursor: usize,
    focused: bool,
    y_offset: u16,
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
        }
    }

    pub fn focus(&mut self) { self.focused = true; }
    pub fn blur(&mut self) { self.focused = false; }
    pub fn selected_index(&self) -> usize { self.cursor }
    pub fn selected_value(&self) -> &str { &self.items[self.cursor] }

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
            KeyCode::Enter => {
                Some(SelectMsg::Selected(self.cursor, self.items[self.cursor].clone()))
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
                    Some(SelectMsg::Selected(self.cursor, self.items[self.cursor].clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let children: Vec<Element<Msg>> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let prefix = if i == self.cursor { "▸ " } else { "  " };
                let text = format!("{}{}", prefix, item);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent { code, modifiers: KeyModifiers::NONE }
    }

    #[test]
    fn initial_state() {
        let select = Select::new(vec!["a", "b", "c"]);
        assert_eq!(select.selected_index(), 0);
        assert_eq!(select.selected_value(), "a");
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
    fn blur_ignores_input() {
        let mut select = Select::new(vec!["a", "b"]);
        select.blur();
        select.handle_key(&key(KeyCode::Down));
        assert_eq!(select.selected_index(), 0);
    }
}
