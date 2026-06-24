//! Core model traits for TEA (The Elm Architecture).
//!
//! Two traits are provided:
//! - [`Model`]: String-based rendering (simple, low-level)
//! - [`ElementModel`]: Element tree rendering (recommended, Flexbox layout)

use crate::cmd::Cmd;
use crate::element::Element;

/// A TEA model with string-based rendering.
///
/// Implement this trait for simple applications that render directly to a string.
/// For most use cases, prefer [`ElementModel`] which provides Flexbox layout.
pub trait Model: Send + 'static {
    type Msg: Send + 'static;

    /// Called once when the program starts. Return a command to perform initialization.
    fn init(&mut self) -> Option<Cmd<Self::Msg>> {
        None
    }

    /// Handle a message and update state. Return an optional command.
    fn update(&mut self, msg: Self::Msg) -> Option<Cmd<Self::Msg>>;

    /// Render the current state to a string.
    fn view(&self) -> String;

    /// Where to place the real terminal cursor as `(column, row)`, or `None` to
    /// keep it hidden. Returning a position shows a normal blinking cursor at the
    /// text insertion point — the correct behaviour for input fields (including
    /// wide CJK glyphs), instead of a faked reverse-video block.
    fn cursor(&self) -> Option<(u16, u16)> {
        None
    }
}

/// A TEA model with Element tree rendering and Flexbox layout.
///
/// This is the recommended trait for building terminal UIs. The `view()` method
/// returns an [`Element`] tree that is laid out using Flexbox and rendered
/// incrementally.
pub trait ElementModel: Send + 'static {
    type Msg: Send + 'static;

    /// Called once when the program starts. Return a command to perform initialization.
    fn init(&mut self) -> Option<Cmd<Self::Msg>> {
        None
    }

    /// Handle a message and update state. Return an optional command.
    fn update(&mut self, msg: Self::Msg) -> Option<Cmd<Self::Msg>>;

    /// Build the UI element tree from current state.
    fn view(&self) -> Element<Self::Msg>;
}
