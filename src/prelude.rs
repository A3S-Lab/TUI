//! Stable convenience imports for application code.
//!
//! The prelude intentionally stays smaller than the full crate root. It exports
//! the types most applications need to define a TEA-style terminal program,
//! build element trees, style text, and work with common events.

pub use crate::chrome::AgentChrome;
pub use crate::cmd::{self, Cmd};
pub use crate::components;
pub use crate::element::{
    AlignItems, BorderStyle, BoxElement, BoxStyle, Dimension, Edges, Element, FlexDirection,
    JustifyContent, Overflow, TextElement, TextStyle, TextWrap,
};
pub use crate::element_program::{ElementProgram, ElementProgramBuilder};
pub use crate::event::{Event, KeyEvent, MouseEvent};
pub use crate::focus::{FocusId, FocusManager};
pub use crate::input::{
    InputCapture, InputCaptureMode, InputHelpEntry, InputRoute, InputRouter, InputScope,
    RoutedInput,
};
pub use crate::interaction::{Activatable, Scrollable, Selectable, Tabbed};
pub use crate::key::{KeyCode, KeyModifiers};
pub use crate::keymap::{KeyBinding, Keymap};
pub use crate::layout::{Constraint, Direction, Layout};
#[cfg(feature = "markdown")]
pub use crate::markdown::Markdown;
pub use crate::model::{ElementModel, Model};
pub use crate::program::{Program, ProgramBuilder};
pub use crate::style::{Align, Border, Color, Style};
pub use crate::theme::{
    BuiltinTheme, ParseBuiltinThemeError, ParseThemeRoleError, Theme, ThemeRole,
};
