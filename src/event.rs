//! Terminal event types (keyboard, mouse, resize, focus).

use crossterm::event::{KeyCode, KeyModifiers, MouseEventKind as CtMouseEventKind};

/// A terminal event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Keyboard input.
    Key(KeyEvent),
    /// Mouse input.
    Mouse(MouseEvent),
    /// Terminal window resized.
    Resize { width: u16, height: u16 },
    /// Terminal gained focus.
    FocusGained,
    /// Terminal lost focus.
    FocusLost,
}

/// A keyboard event with key code and modifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

/// A mouse event with position and button state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: KeyModifiers,
}

/// The type of mouse action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MouseEventKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    Moved,
    ScrollDown,
    ScrollUp,
    ScrollLeft,
    ScrollRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl From<crossterm::event::Event> for Event {
    fn from(event: crossterm::event::Event) -> Self {
        match event {
            crossterm::event::Event::Key(key) => Event::Key(KeyEvent {
                code: key.code,
                modifiers: key.modifiers,
            }),
            crossterm::event::Event::Mouse(mouse) => Event::Mouse(MouseEvent {
                kind: convert_mouse_kind(mouse.kind),
                column: mouse.column,
                row: mouse.row,
                modifiers: mouse.modifiers,
            }),
            crossterm::event::Event::Resize(w, h) => Event::Resize {
                width: w,
                height: h,
            },
            crossterm::event::Event::FocusGained => Event::FocusGained,
            crossterm::event::Event::FocusLost => Event::FocusLost,
            crossterm::event::Event::Paste(_) => Event::Key(KeyEvent {
                code: KeyCode::Null,
                modifiers: KeyModifiers::NONE,
            }),
        }
    }
}

fn convert_mouse_button(button: crossterm::event::MouseButton) -> MouseButton {
    match button {
        crossterm::event::MouseButton::Left => MouseButton::Left,
        crossterm::event::MouseButton::Right => MouseButton::Right,
        crossterm::event::MouseButton::Middle => MouseButton::Middle,
    }
}

fn convert_mouse_kind(kind: CtMouseEventKind) -> MouseEventKind {
    match kind {
        CtMouseEventKind::Down(b) => MouseEventKind::Down(convert_mouse_button(b)),
        CtMouseEventKind::Up(b) => MouseEventKind::Up(convert_mouse_button(b)),
        CtMouseEventKind::Drag(b) => MouseEventKind::Drag(convert_mouse_button(b)),
        CtMouseEventKind::Moved => MouseEventKind::Moved,
        CtMouseEventKind::ScrollDown => MouseEventKind::ScrollDown,
        CtMouseEventKind::ScrollUp => MouseEventKind::ScrollUp,
        CtMouseEventKind::ScrollLeft => MouseEventKind::ScrollLeft,
        CtMouseEventKind::ScrollRight => MouseEventKind::ScrollRight,
    }
}
