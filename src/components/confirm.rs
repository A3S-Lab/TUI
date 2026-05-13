use crate::element::{BoxElement, BorderStyle, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseEvent, MouseEventKind, MouseButton};
use crate::style::Color;
use crossterm::event::KeyCode;

/// A confirmation dialog with Yes/No options.
pub struct Confirm {
    message: String,
    selected: bool,
    yes_label: String,
    no_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmMsg {
    Confirmed,
    Cancelled,
}

impl Confirm {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            selected: true,
            yes_label: "Yes".to_string(),
            no_label: "No".to_string(),
        }
    }

    pub fn with_labels(mut self, yes: impl Into<String>, no: impl Into<String>) -> Self {
        self.yes_label = yes.into();
        self.no_label = no.into();
        self
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<ConfirmMsg> {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Tab => {
                self.selected = !self.selected;
                None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.selected = !self.selected;
                None
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => Some(ConfirmMsg::Confirmed),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Some(ConfirmMsg::Cancelled),
            KeyCode::Enter => {
                if self.selected {
                    Some(ConfirmMsg::Confirmed)
                } else {
                    Some(ConfirmMsg::Cancelled)
                }
            }
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<ConfirmMsg> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if self.selected {
                    Some(ConfirmMsg::Confirmed)
                } else {
                    Some(ConfirmMsg::Cancelled)
                }
            }
            _ => None,
        }
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let yes_el = if self.selected {
            Element::Text(
                TextElement::new(format!(" [{}] ", self.yes_label))
                    .bold()
                    .fg(Color::White)
                    .bg(Color::Green),
            )
        } else {
            Element::Text(
                TextElement::new(format!("  {}  ", self.yes_label)).fg(Color::BrightBlack),
            )
        };

        let no_el = if !self.selected {
            Element::Text(
                TextElement::new(format!(" [{}] ", self.no_label))
                    .bold()
                    .fg(Color::White)
                    .bg(Color::Red),
            )
        } else {
            Element::Text(
                TextElement::new(format!("  {}  ", self.no_label)).fg(Color::BrightBlack),
            )
        };

        let buttons = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .gap(2)
                .child(yes_el)
                .child(no_el),
        );

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .border(BorderStyle::Rounded)
                .border_color(Color::BrightBlack)
                .padding(1)
                .gap(1)
                .child(Element::Text(TextElement::new(&self.message).bold()))
                .child(buttons),
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
    fn default_selects_yes() {
        let confirm = Confirm::new("Delete?");
        assert!(confirm.selected);
    }

    #[test]
    fn enter_confirms_when_yes_selected() {
        let mut confirm = Confirm::new("Delete?");
        let msg = confirm.handle_key(&key(KeyCode::Enter));
        assert_eq!(msg, Some(ConfirmMsg::Confirmed));
    }

    #[test]
    fn enter_cancels_when_no_selected() {
        let mut confirm = Confirm::new("Delete?");
        confirm.handle_key(&key(KeyCode::Right));
        let msg = confirm.handle_key(&key(KeyCode::Enter));
        assert_eq!(msg, Some(ConfirmMsg::Cancelled));
    }

    #[test]
    fn y_key_confirms() {
        let mut confirm = Confirm::new("Sure?");
        let msg = confirm.handle_key(&key(KeyCode::Char('y')));
        assert_eq!(msg, Some(ConfirmMsg::Confirmed));
    }

    #[test]
    fn n_key_cancels() {
        let mut confirm = Confirm::new("Sure?");
        let msg = confirm.handle_key(&key(KeyCode::Char('n')));
        assert_eq!(msg, Some(ConfirmMsg::Cancelled));
    }

    #[test]
    fn esc_cancels() {
        let mut confirm = Confirm::new("Sure?");
        let msg = confirm.handle_key(&key(KeyCode::Esc));
        assert_eq!(msg, Some(ConfirmMsg::Cancelled));
    }

    #[test]
    fn tab_toggles_selection() {
        let mut confirm = Confirm::new("Sure?");
        assert!(confirm.selected);
        confirm.handle_key(&key(KeyCode::Tab));
        assert!(!confirm.selected);
        confirm.handle_key(&key(KeyCode::Tab));
        assert!(confirm.selected);
    }

    #[test]
    fn custom_labels() {
        let confirm = Confirm::new("Proceed?")
            .with_labels("Continue", "Abort");
        assert_eq!(confirm.yes_label, "Continue");
        assert_eq!(confirm.no_label, "Abort");
    }
}
