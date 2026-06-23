//! Configurable key binding system for mapping keys to actions.

use crate::event::KeyEvent;
use crossterm::event::{KeyCode, KeyModifiers};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    pub fn ctrl(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }

    pub fn alt(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::ALT,
        }
    }

    pub fn shift(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::SHIFT,
        }
    }

    pub fn with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn matches(&self, event: &KeyEvent) -> bool {
        self.code == event.code && self.modifiers == event.modifiers
    }

    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("Ctrl".to_string());
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt".to_string());
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("Shift".to_string());
        }
        parts.push(key_name(self.code));
        parts.join("+")
    }
}

impl From<&KeyEvent> for KeyBinding {
    fn from(event: &KeyEvent) -> Self {
        Self {
            code: event.code,
            modifiers: event.modifiers,
        }
    }
}

pub struct Keymap<A: Clone> {
    bindings: HashMap<KeyBinding, (A, String)>,
}

impl<A: Clone> Keymap<A> {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn bind(mut self, key: KeyBinding, action: A, description: &str) -> Self {
        self.bindings.insert(key, (action, description.to_string()));
        self
    }

    pub fn register(&mut self, key: KeyBinding, action: A, description: &str) {
        self.bindings.insert(key, (action, description.to_string()));
    }

    pub fn unbind(&mut self, key: &KeyBinding) {
        self.bindings.remove(key);
    }

    pub fn resolve(&self, event: &KeyEvent) -> Option<A> {
        let binding = KeyBinding::from(event);
        self.bindings
            .get(&binding)
            .map(|(action, _)| action.clone())
    }

    pub fn help(&self) -> Vec<(String, String)> {
        self.bindings
            .iter()
            .map(|(key, (_, desc))| (key.display(), desc.clone()))
            .collect()
    }
}

impl<A: Clone> Default for Keymap<A> {
    fn default() -> Self {
        Self::new()
    }
}

fn key_name(code: KeyCode) -> String {
    match code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::F(n) => format!("F{}", n),
        _ => "?".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Action {
        Quit,
        Save,
        Help,
    }

    #[test]
    fn keybinding_matches_event() {
        let binding = KeyBinding::new(KeyCode::Char('q'));
        let event = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        };
        assert!(binding.matches(&event));
    }

    #[test]
    fn keybinding_ctrl_modifier() {
        let binding = KeyBinding::ctrl(KeyCode::Char('c'));
        let event = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        };
        assert!(binding.matches(&event));

        let plain = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::NONE,
        };
        assert!(!binding.matches(&plain));
    }

    #[test]
    fn keybinding_display() {
        assert_eq!(KeyBinding::new(KeyCode::Char('q')).display(), "q");
        assert_eq!(KeyBinding::ctrl(KeyCode::Char('c')).display(), "Ctrl+c");
        assert_eq!(KeyBinding::alt(KeyCode::Char('x')).display(), "Alt+x");
        assert_eq!(KeyBinding::new(KeyCode::F(1)).display(), "F1");
    }

    #[test]
    fn keymap_resolve() {
        let keymap = Keymap::new()
            .bind(KeyBinding::new(KeyCode::Char('q')), Action::Quit, "Quit")
            .bind(KeyBinding::ctrl(KeyCode::Char('s')), Action::Save, "Save");

        let event = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(keymap.resolve(&event), Some(Action::Quit));

        let event2 = KeyEvent {
            code: KeyCode::Char('s'),
            modifiers: KeyModifiers::CONTROL,
        };
        assert_eq!(keymap.resolve(&event2), Some(Action::Save));
    }

    #[test]
    fn keymap_resolve_unbound_returns_none() {
        let keymap: Keymap<Action> = Keymap::new();
        let event = KeyEvent {
            code: KeyCode::Char('z'),
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(keymap.resolve(&event), None);
    }

    #[test]
    fn keymap_unbind() {
        let mut keymap =
            Keymap::new().bind(KeyBinding::new(KeyCode::Char('q')), Action::Quit, "Quit");
        keymap.unbind(&KeyBinding::new(KeyCode::Char('q')));
        let event = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(keymap.resolve(&event), None);
    }

    #[test]
    fn keymap_help_lists_bindings() {
        let keymap = Keymap::new()
            .bind(
                KeyBinding::new(KeyCode::Char('q')),
                Action::Quit,
                "Quit app",
            )
            .bind(
                KeyBinding::new(KeyCode::Char('?')),
                Action::Help,
                "Show help",
            );
        let help = keymap.help();
        assert_eq!(help.len(), 2);
    }
}
