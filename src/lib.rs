//! A3S TUI - TEA (The Elm Architecture) framework for terminal user interfaces.
//!
//! This library provides a declarative way to build terminal applications using
//! The Elm Architecture pattern, with Flexbox layout powered by Taffy and
//! incremental rendering for optimal performance.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use a3s_tui::{cmd, col, text, Element, ElementModel, ElementProgramBuilder};
//! use a3s_tui::{Event, KeyCode, TextElement};
//! use a3s_tui::style::Color;
//!
//! struct Counter { count: i64 }
//!
//! enum Msg {
//!     Increment,
//!     Decrement,
//!     Quit,
//! }
//!
//! impl From<Event> for Msg {
//!     fn from(event: Event) -> Self {
//!         match event {
//!             Event::Key(key) if key.code == KeyCode::Up => Msg::Increment,
//!             Event::Key(key) if key.code == KeyCode::Down => Msg::Decrement,
//!             Event::Key(key) if key.code == KeyCode::Char('q') => Msg::Quit,
//!             _ => Msg::Increment,
//!         }
//!     }
//! }
//!
//! impl ElementModel for Counter {
//!     type Msg = Msg;
//!
//!     fn update(&mut self, msg: Msg) -> Option<cmd::Cmd<Msg>> {
//!         match msg {
//!             Msg::Increment => { self.count += 1; None }
//!             Msg::Decrement => { self.count -= 1; None }
//!             Msg::Quit => Some(cmd::quit()),
//!         }
//!     }
//!
//!     fn view(&self) -> Element<Msg> {
//!         col![
//!             text!(""),
//!             Element::Text(
//!                 TextElement::new(format!("Counter: {}", self.count))
//!                     .bold()
//!                     .fg(Color::Cyan)
//!             ),
//!         ]
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> std::io::Result<()> {
//!     ElementProgramBuilder::new(Counter { count: 0 })
//!         .with_alt_screen()
//!         .with_fps(30)
//!         .run()
//!         .await
//! }
//! ```

pub mod cmd;
pub mod components;
pub mod diff;
pub mod element;
pub mod element_program;
pub mod event;
pub mod focus;
pub mod grid;
pub mod key;
pub mod keymap;
pub mod layout;
pub mod layout_engine;
#[macro_use]
pub mod macros;
pub mod animation;
pub mod markdown;
pub mod model;
pub mod paint;
pub mod program;
pub mod renderer;
pub mod streaming;
pub mod style;
pub mod terminal;
pub mod theme;

pub use animation::{Easing, FrameAnimation, Transition};
pub use cmd::Cmd;
pub use element::{
    AlignItems, BorderStyle, BoxElement, BoxStyle, Dimension, Edges, Element, FlexDirection,
    JustifyContent, Overflow, TextElement, TextStyle, TextWrap,
};
pub use element_program::{ElementProgram, ElementProgramBuilder};
pub use event::{Event, KeyEvent, MouseEvent};
pub use focus::{FocusId, FocusManager};
pub use grid::{Cell, CellStyle, Grid};
pub use key::{KeyCode, KeyModifiers};
pub use keymap::{KeyBinding, Keymap};
pub use layout::{Constraint, Direction, Layout};
pub use model::{ElementModel, Model};
pub use program::{Program, ProgramBuilder};
pub use style::{Align, Border, Color, Style};
pub use theme::Theme;
