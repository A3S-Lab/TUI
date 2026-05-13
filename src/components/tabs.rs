use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseEvent, MouseEventKind, MouseButton};
use crate::style::Color;
use crossterm::event::KeyCode;

pub struct Tabs {
    labels: Vec<String>,
    active: usize,
    focused: bool,
    x_offset: u16,
}

#[derive(Debug, Clone)]
pub enum TabsMsg {
    Changed(usize),
}

impl Tabs {
    pub fn new(labels: Vec<impl Into<String>>) -> Self {
        Self {
            labels: labels.into_iter().map(|l| l.into()).collect(),
            active: 0,
            focused: true,
            x_offset: 0,
        }
    }

    pub fn focus(&mut self) { self.focused = true; }
    pub fn blur(&mut self) { self.focused = false; }
    pub fn active(&self) -> usize { self.active }
    pub fn set_active(&mut self, idx: usize) {
        if idx < self.labels.len() {
            self.active = idx;
        }
    }

    /// Set the horizontal offset for mouse click calculations.
    pub fn set_x_offset(&mut self, x: u16) {
        self.x_offset = x;
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<TabsMsg> {
        if !self.focused { return None; }
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                if self.active > 0 {
                    self.active -= 1;
                    Some(TabsMsg::Changed(self.active))
                } else { None }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.active + 1 < self.labels.len() {
                    self.active += 1;
                    Some(TabsMsg::Changed(self.active))
                } else { None }
            }
            _ => None,
        }
    }

    /// Handle mouse click to select a tab.
    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<TabsMsg> {
        if !self.focused { return None; }
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let click_x = mouse.column.saturating_sub(self.x_offset) as usize;
                let mut x = 0;
                for (i, label) in self.labels.iter().enumerate() {
                    let tab_width = label.len() + 2; // " label "
                    if click_x >= x && click_x < x + tab_width {
                        self.active = i;
                        return Some(TabsMsg::Changed(i));
                    }
                    x += tab_width + 1; // +1 for separator space
                }
                None
            }
            _ => None,
        }
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children: Vec<Element<Msg>> = Vec::new();
        for (i, label) in self.labels.iter().enumerate() {
            let padded = format!(" {} ", label);
            if i == self.active {
                children.push(Element::Text(
                    TextElement::new(padded).bold().fg(Color::BrightWhite).bg(Color::Blue),
                ));
            } else {
                children.push(Element::Text(
                    TextElement::new(padded).fg(Color::BrightBlack),
                ));
            }
            if i + 1 < self.labels.len() {
                children.push(Element::Text(TextElement::new(" ").fg(Color::BrightBlack)));
            }
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
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
        let tabs = Tabs::new(vec!["A", "B", "C"]);
        assert_eq!(tabs.active(), 0);
    }

    #[test]
    fn navigate_right() {
        let mut tabs = Tabs::new(vec!["A", "B", "C"]);
        tabs.handle_key(&key(KeyCode::Right));
        assert_eq!(tabs.active(), 1);
        tabs.handle_key(&key(KeyCode::Char('l')));
        assert_eq!(tabs.active(), 2);
    }

    #[test]
    fn navigate_left() {
        let mut tabs = Tabs::new(vec!["A", "B", "C"]);
        tabs.handle_key(&key(KeyCode::Right));
        tabs.handle_key(&key(KeyCode::Right));
        tabs.handle_key(&key(KeyCode::Left));
        assert_eq!(tabs.active(), 1);
    }

    #[test]
    fn bounds_check() {
        let mut tabs = Tabs::new(vec!["A", "B"]);
        tabs.handle_key(&key(KeyCode::Left));
        assert_eq!(tabs.active(), 0);
        tabs.handle_key(&key(KeyCode::Right));
        tabs.handle_key(&key(KeyCode::Right));
        assert_eq!(tabs.active(), 1);
    }

    #[test]
    fn set_active() {
        let mut tabs = Tabs::new(vec!["A", "B", "C"]);
        tabs.set_active(2);
        assert_eq!(tabs.active(), 2);
        tabs.set_active(99);
        assert_eq!(tabs.active(), 2);
    }

    #[test]
    fn returns_changed_msg() {
        let mut tabs = Tabs::new(vec!["A", "B"]);
        let msg = tabs.handle_key(&key(KeyCode::Right));
        assert!(matches!(msg, Some(TabsMsg::Changed(1))));
    }
}
