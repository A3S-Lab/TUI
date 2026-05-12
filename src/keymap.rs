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
        self.bindings.get(&binding).map(|(action, _)| action.clone())
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
