use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::KeyEvent;
use crate::style::Color;
use crossterm::event::KeyCode;

pub struct MultiSelect {
    items: Vec<String>,
    cursor: usize,
    checked: Vec<bool>,
    focused: bool,
}

#[derive(Debug, Clone)]
pub enum MultiSelectMsg {
    Toggle(usize),
    Submit(Vec<usize>),
}

impl MultiSelect {
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        let items: Vec<String> = items.into_iter().map(|i| i.into()).collect();
        let checked = vec![false; items.len()];
        Self {
            items,
            cursor: 0,
            checked,
            focused: true,
        }
    }

    pub fn focus(&mut self) { self.focused = true; }
    pub fn blur(&mut self) { self.focused = false; }

    pub fn selected_indices(&self) -> Vec<usize> {
        self.checked.iter().enumerate().filter_map(|(i, &b)| if b { Some(i) } else { None }).collect()
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<MultiSelectMsg> {
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
            KeyCode::Char(' ') => {
                self.checked[self.cursor] = !self.checked[self.cursor];
                Some(MultiSelectMsg::Toggle(self.cursor))
            }
            KeyCode::Enter => {
                Some(MultiSelectMsg::Submit(self.selected_indices()))
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
                let cursor_marker = if i == self.cursor { "▸" } else { " " };
                let check = if self.checked[i] { "[x]" } else { "[ ]" };
                let text = format!("{} {} {}", cursor_marker, check, item);
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
        let ms = MultiSelect::new(vec!["a", "b", "c"]);
        assert!(ms.selected_indices().is_empty());
    }

    #[test]
    fn toggle_with_space() {
        let mut ms = MultiSelect::new(vec!["a", "b", "c"]);
        ms.handle_key(&key(KeyCode::Char(' ')));
        assert_eq!(ms.selected_indices(), vec![0]);
        ms.handle_key(&key(KeyCode::Down));
        ms.handle_key(&key(KeyCode::Char(' ')));
        assert_eq!(ms.selected_indices(), vec![0, 1]);
    }

    #[test]
    fn untoggle() {
        let mut ms = MultiSelect::new(vec!["a", "b"]);
        ms.handle_key(&key(KeyCode::Char(' ')));
        ms.handle_key(&key(KeyCode::Char(' ')));
        assert!(ms.selected_indices().is_empty());
    }

    #[test]
    fn submit_returns_selected() {
        let mut ms = MultiSelect::new(vec!["x", "y", "z"]);
        ms.handle_key(&key(KeyCode::Char(' ')));
        ms.handle_key(&key(KeyCode::Down));
        ms.handle_key(&key(KeyCode::Down));
        ms.handle_key(&key(KeyCode::Char(' ')));
        let msg = ms.handle_key(&key(KeyCode::Enter));
        assert!(matches!(msg, Some(MultiSelectMsg::Submit(v)) if v == vec![0, 2]));
    }

    #[test]
    fn navigation_bounds() {
        let mut ms = MultiSelect::new(vec!["a", "b"]);
        ms.handle_key(&key(KeyCode::Up));
        assert_eq!(ms.cursor, 0);
        ms.handle_key(&key(KeyCode::Down));
        ms.handle_key(&key(KeyCode::Down));
        assert_eq!(ms.cursor, 1);
    }

    #[test]
    fn blur_ignores_input() {
        let mut ms = MultiSelect::new(vec!["a"]);
        ms.blur();
        ms.handle_key(&key(KeyCode::Char(' ')));
        assert!(ms.selected_indices().is_empty());
    }
}
