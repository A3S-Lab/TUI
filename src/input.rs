//! Input routing for global, focused, and captured command scopes.
//!
//! [`InputRouter`] builds on [`Keymap`](crate::keymap::Keymap) and
//! [`FocusManager`](crate::focus::FocusManager). It lets application shells keep
//! global shortcuts, focused component shortcuts, and modal/overlay shortcuts in
//! one predictable resolution order.

use crate::event::{Event, KeyEvent};
use crate::focus::FocusId;
use crate::keymap::{KeyBinding, Keymap};
use std::collections::HashMap;

/// A namespace for input bindings outside the global keymap.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputScope {
    /// Bindings owned by a focusable component.
    Focus(FocusId),
    /// Bindings owned by an overlay, modal, mode, or named tool surface.
    Named(String),
}

impl InputScope {
    /// Create a scope for a focusable component.
    pub fn focused(id: FocusId) -> Self {
        Self::Focus(id)
    }

    /// Create a named scope for an overlay, modal, mode, or tool surface.
    pub fn named(name: impl Into<String>) -> Self {
        Self::Named(name.into())
    }
}

impl From<FocusId> for InputScope {
    fn from(id: FocusId) -> Self {
        Self::focused(id)
    }
}

impl From<&str> for InputScope {
    fn from(name: &str) -> Self {
        Self::named(name)
    }
}

impl From<String> for InputScope {
    fn from(name: String) -> Self {
        Self::named(name)
    }
}

/// How a captured scope behaves when it does not handle a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputCaptureMode {
    /// Stop routing when the captured scope does not handle the key.
    Exclusive,
    /// Continue routing to older captures, focused bindings, and globals.
    Passthrough,
}

/// A scope currently capturing input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputCapture {
    scope: InputScope,
    mode: InputCaptureMode,
}

impl InputCapture {
    /// Scope that owns this capture.
    pub fn scope(&self) -> &InputScope {
        &self.scope
    }

    /// Capture behavior for unhandled keys.
    pub fn mode(&self) -> InputCaptureMode {
        self.mode
    }
}

/// Where a routed action was resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputRoute {
    /// A captured scope handled the key.
    Captured(InputScope),
    /// The currently focused component handled the key.
    Focus(FocusId),
    /// The global keymap handled the key.
    Global,
}

/// A resolved input action and its source route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedInput<A> {
    /// Action associated with the key binding.
    pub action: A,
    /// Route that produced the action.
    pub route: InputRoute,
}

/// A help entry from a route that is currently eligible for input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputHelpEntry {
    /// Route that owns this binding.
    pub route: InputRoute,
    /// Human-readable key name, such as `Ctrl+c`.
    pub key: String,
    /// Binding description.
    pub description: String,
}

/// Routes key events through captured, focused, and global keymaps.
///
/// Resolution order is:
///
/// 1. Captured scopes, newest first.
/// 2. Bindings for the current focused component.
/// 3. Global bindings.
///
/// Exclusive captures stop routing when they do not handle a key, which is
/// useful for blocking modals. Passthrough captures let unhandled keys continue
/// to older layers, which is useful for temporary modes or non-blocking
/// overlays.
pub struct InputRouter<A: Clone> {
    global: Keymap<A>,
    focused: HashMap<FocusId, Keymap<A>>,
    named: HashMap<String, Keymap<A>>,
    captures: Vec<InputCapture>,
}

impl<A: Clone> InputRouter<A> {
    /// Create an empty input router.
    pub fn new() -> Self {
        Self {
            global: Keymap::new(),
            focused: HashMap::new(),
            named: HashMap::new(),
            captures: Vec::new(),
        }
    }

    /// Add a global binding and return the router.
    pub fn bind_global(mut self, key: KeyBinding, action: A, description: &str) -> Self {
        self.register_global(key, action, description);
        self
    }

    /// Add a focused component binding and return the router.
    pub fn bind_focus(
        mut self,
        id: FocusId,
        key: KeyBinding,
        action: A,
        description: &str,
    ) -> Self {
        self.register_focus(id, key, action, description);
        self
    }

    /// Add a named or focus-scope binding and return the router.
    pub fn bind_scope<S>(mut self, scope: S, key: KeyBinding, action: A, description: &str) -> Self
    where
        S: Into<InputScope>,
    {
        self.register_scope(scope, key, action, description);
        self
    }

    /// Register a global binding.
    pub fn register_global(&mut self, key: KeyBinding, action: A, description: &str) {
        self.global.register(key, action, description);
    }

    /// Register a binding for a focusable component.
    pub fn register_focus(&mut self, id: FocusId, key: KeyBinding, action: A, description: &str) {
        self.focused
            .entry(id)
            .or_default()
            .register(key, action, description);
    }

    /// Register a binding for a named or focus scope.
    pub fn register_scope<S>(&mut self, scope: S, key: KeyBinding, action: A, description: &str)
    where
        S: Into<InputScope>,
    {
        match scope.into() {
            InputScope::Focus(id) => self.register_focus(id, key, action, description),
            InputScope::Named(name) => {
                self.named
                    .entry(name)
                    .or_default()
                    .register(key, action, description);
            }
        }
    }

    /// Push an exclusive capture for a scope.
    pub fn push_capture<S>(&mut self, scope: S)
    where
        S: Into<InputScope>,
    {
        self.push_capture_with_mode(scope, InputCaptureMode::Exclusive);
    }

    /// Push a capture with explicit unhandled-key behavior.
    pub fn push_capture_with_mode<S>(&mut self, scope: S, mode: InputCaptureMode)
    where
        S: Into<InputScope>,
    {
        self.captures.push(InputCapture {
            scope: scope.into(),
            mode,
        });
    }

    /// Remove the newest capture and return it.
    pub fn pop_capture(&mut self) -> Option<InputCapture> {
        self.captures.pop()
    }

    /// Remove the newest capture for a scope.
    pub fn remove_capture<S>(&mut self, scope: S) -> bool
    where
        S: Into<InputScope>,
    {
        let scope = scope.into();
        if let Some(pos) = self
            .captures
            .iter()
            .rposition(|capture| capture.scope == scope)
        {
            self.captures.remove(pos);
            true
        } else {
            false
        }
    }

    /// Remove all active captures.
    pub fn clear_captures(&mut self) {
        self.captures.clear();
    }

    /// Return the currently active capture, if any.
    pub fn active_capture(&self) -> Option<&InputCapture> {
        self.captures.last()
    }

    /// Resolve a key event against the current captures, focus, and globals.
    pub fn resolve_key(&self, key: &KeyEvent, focused: Option<FocusId>) -> Option<RoutedInput<A>> {
        for capture in self.captures.iter().rev() {
            if let Some(keymap) = self.keymap_for_scope(&capture.scope) {
                if let Some(action) = keymap.resolve(key) {
                    return Some(RoutedInput {
                        action,
                        route: InputRoute::Captured(capture.scope.clone()),
                    });
                }
            }

            if capture.mode == InputCaptureMode::Exclusive {
                return None;
            }
        }

        if let Some(id) = focused {
            if let Some(keymap) = self.focused.get(&id) {
                if let Some(action) = keymap.resolve(key) {
                    return Some(RoutedInput {
                        action,
                        route: InputRoute::Focus(id),
                    });
                }
            }
        }

        self.global.resolve(key).map(|action| RoutedInput {
            action,
            route: InputRoute::Global,
        })
    }

    /// Resolve a terminal event. Non-key events are ignored.
    pub fn resolve_event(&self, event: &Event, focused: Option<FocusId>) -> Option<RoutedInput<A>> {
        match event {
            Event::Key(key) => self.resolve_key(key, focused),
            _ => None,
        }
    }

    /// Help entries for global bindings.
    pub fn global_help(&self) -> Vec<(String, String)> {
        self.global.help()
    }

    /// Help entries for a focusable component.
    pub fn focus_help(&self, id: FocusId) -> Vec<(String, String)> {
        self.focused.get(&id).map(Keymap::help).unwrap_or_default()
    }

    /// Help entries for a named or focus scope.
    pub fn scope_help<S>(&self, scope: S) -> Vec<(String, String)>
    where
        S: Into<InputScope>,
    {
        self.keymap_for_scope(&scope.into())
            .map(Keymap::help)
            .unwrap_or_default()
    }

    /// Help entries in the same order that key routing would consider them.
    pub fn active_help(&self, focused: Option<FocusId>) -> Vec<InputHelpEntry> {
        let mut entries = Vec::new();

        for capture in self.captures.iter().rev() {
            entries.extend(self.help_entries_for_scope(
                &capture.scope,
                InputRoute::Captured(capture.scope.clone()),
            ));
            if capture.mode == InputCaptureMode::Exclusive {
                return entries;
            }
        }

        if let Some(id) = focused {
            entries
                .extend(self.help_entries_for_keymap(InputRoute::Focus(id), self.focused.get(&id)));
        }

        entries.extend(self.help_entries_for_keymap(InputRoute::Global, Some(&self.global)));
        entries
    }

    fn keymap_for_scope(&self, scope: &InputScope) -> Option<&Keymap<A>> {
        match scope {
            InputScope::Focus(id) => self.focused.get(id),
            InputScope::Named(name) => self.named.get(name),
        }
    }

    fn help_entries_for_scope(&self, scope: &InputScope, route: InputRoute) -> Vec<InputHelpEntry> {
        self.help_entries_for_keymap(route, self.keymap_for_scope(scope))
    }

    fn help_entries_for_keymap(
        &self,
        route: InputRoute,
        keymap: Option<&Keymap<A>>,
    ) -> Vec<InputHelpEntry> {
        keymap
            .map(|keymap| {
                keymap
                    .help()
                    .into_iter()
                    .map(|(key, description)| InputHelpEntry {
                        route: route.clone(),
                        key,
                        description,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl<A: Clone> Default for InputRouter<A> {
    fn default() -> Self {
        Self::new()
    }
}
