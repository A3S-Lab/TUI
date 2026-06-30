# A3S TUI

**TEA (The Elm Architecture) framework for terminal user interfaces**

A3S TUI is a Rust library for building terminal applications using The Elm Architecture pattern. It combines declarative UI with Flexbox layout, incremental rendering, and a rich component library.

[![crates.io](https://img.shields.io/crates/v/a3s-tui)](https://crates.io/crates/a3s-tui)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

---

## Why

Most terminal UI libraries force you to manage state, layout, and rendering manually. A3S TUI brings modern UI patterns to the terminal:

- **TEA Architecture** — predictable state management with Model-Update-View
- **Declarative UI** — describe what you want, not how to draw it
- **Flexbox Layout** — CSS-like layout powered by [Taffy](https://github.com/DioxusLabs/taffy)
- **Incremental Rendering** — only redraw what changed
- **Rich Components** — 58 ready-to-use components (tables, modals, help panels, text editors, etc.)

---

## Quick Start

Add to `Cargo.toml`:

```toml
[dependencies]
a3s-tui = "0.1"
tokio = { version = "1", features = ["full"] }
```

Create a counter app:

```rust
use a3s_tui::{cmd, col, text, Element, ElementModel, ElementProgramBuilder};
use a3s_tui::{Event, KeyCode, TextElement};
use a3s_tui::style::Color;

struct Counter { count: i64 }

enum Msg {
    Increment,
    Decrement,
    Quit,
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match event {
            Event::Key(key) if key.code == KeyCode::Up => Msg::Increment,
            Event::Key(key) if key.code == KeyCode::Down => Msg::Decrement,
            Event::Key(key) if key.code == KeyCode::Char('q') => Msg::Quit,
            _ => Msg::Increment, // fallback
        }
    }
}

impl ElementModel for Counter {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Option<cmd::Cmd<Msg>> {
        match msg {
            Msg::Increment => { self.count += 1; None }
            Msg::Decrement => { self.count -= 1; None }
            Msg::Quit => Some(cmd::quit()),
        }
    }

    fn view(&self) -> Element<Msg> {
        col![
            text!(""),
            Element::Text(
                TextElement::new(format!("Counter: {}", self.count))
                    .bold()
                    .fg(Color::Cyan)
            ),
            text!(""),
            text!("Up/Down to change | q to quit").dim(),
        ]
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    ElementProgramBuilder::new(Counter { count: 0 })
        .with_alt_screen()
        .with_fps(30)
        .run()
        .await
}
```

Run with `cargo run --example counter_element`.

---

## Features

### Architecture

- **TEA Pattern** — Model-Update-View cycle with immutable state
- **Element Tree** — Virtual DOM-like tree structure for declarative UI
- **Taffy Flexbox** — CSS Flexbox layout engine (flex-direction, gap, padding, align-items, justify-content)
- **Incremental Rendering** — Line-diff algorithm minimizes terminal redraws
- **Async Runtime** — Non-blocking event loop powered by Tokio

### Components

| Component | Description |
|-----------|-------------|
| `ActivityBlock` | In-flight activity line with optional live output tail |
| `Alert` | Colored alerts (Success/Info/Warning/Error) |
| `Badge` | Inline status badges |
| `Breadcrumb` | Hierarchical path navigation |
| `Checklist` | Status-aware task/TODO list |
| `ChipStrip` | Compact colored chip strip with active chip styling |
| `ChoicePrompt` | Numbered action picker for approvals and command choices |
| `Confirm` | Keyboard, mouse, and line-rendered confirmation prompt |
| `ConnectorBlock` | Connector-led compact output and task-summary rows |
| `CursorLine` | Display-width-aware editor line with a block cursor |
| `DataTable` | Responsive, scrollable data table |
| `DetailPanel` | Compact selected-row details with metadata and actions |
| `DiffView` | Unified diff renderer for edits, git panels, and tool output |
| `Divider` | Horizontal/vertical separators |
| `GitPanel` | Git status/log panel with selectable files, commits, diffs, and commit input |
| `GutterBlock` | Transcript/message block with marker gutter and optional bubble background |
| `HelpPanel` | Grouped shortcut and command help |
| `InputBorder` | Input-area border line with context, effort, and ribbon variants |
| `KeyValue` | Labeled metadata rows |
| `LevelSlider` | Discrete level slider with tick labels and selected marker |
| `List` | Scrollable list with selection |
| `LogView` | Scrollable log/output panel with loading and empty states |
| `MenuPanel` | Titled scroll-aware menu for command palettes and overlays |
| `Meter` | Compact value meter |
| `MetricTrend` | Metric with trend visualization |
| `ModeLine` | Current mode row with shortcut hints |
| `MultiSelect` | Multi-selection list with checkboxes |
| `OutputBlock` | Status-marked transcript/output block with tail preview |
| `Paragraph` | Width-aware paragraph wrapping |
| `PreviewPanel` | Selectable item list with a live preview section |
| `Progress` | Progress bar |
| `PromptLine` | Prompt-prefixed input text with aligned continuation rows |
| `Scrollbar` | Scrollbar indicator |
| `Select` | Single-selection dropdown |
| `SessionStatus` | Agent/session footer row with context and live chips |
| `ShimmerText` | Animated gliding highlight for activity text |
| `SideNotePanel` | Compact side-channel question and answer panel |
| `Sparkline` | Inline trend chart |
| `SplitPane` | Two-column panel for IDE, git, memory, and detail views |
| `Spinner` | Loading animation |
| `StatusBar` | Bottom status bar |
| `SubagentTracker` | Parallel subagent/background work tracker |
| `Table` | Data table with headers |
| `Tabs` | Tab navigation with metadata and per-tab accents |
| `TabbedMenuPanel` | Colored tab strip with a scroll-aware selected list |
| `TaskQueue` | Pinned running and queued task panel |
| `TextInput` | Single-line text input |
| `TextOverlay` | Compose transient overlay rows into a rendered text frame |
| `Textarea` | Multi-line text editor with scrolling |
| `Timeline` | Sectioned timeline with colored nodes and selected-row highlighting |
| `ToolLogView` | Completed tool/command history with args and indented output |
| `ToastManager` | Transient notifications |
| `Tree` | Expandable tree view |
| `TreePicker` | Selectable, scroll-aware file and hierarchy picker |
| `Modal` | Overlay dialog |
| `Viewport` | Scrollable content container with reusable text-selection helpers |
| `WelcomeBanner` | First-run mascot/art banner with metadata, tips, and notices |
| `WrappedPrefixBlock` | Wrapped callout/transcript block with aligned continuation prefix |

### Layout & Styling

- **Flexbox Layout** — `FlexDirection`, `AlignItems`, `JustifyContent`
- **Dimensions** — `Auto`, `Points(f32)`, `Percent(f32)`
- **Spacing** — `padding`, `margin`, `gap`
- **Borders** — `Single`, `Double`, `Rounded`, `Thick`
- **Colors** — 16 ANSI colors + RGB support
- **Text Styles** — bold, italic, underline, dim, strikethrough

### Advanced Features

- **Markdown Rendering** — Full CommonMark support with syntax highlighting (via `syntect`)
- **Streaming Content** — Real-time text streaming (perfect for LLM outputs)
- **Keymap System** — Vim-like key bindings
- **Focus Management** — Tab navigation between components
- **Mouse Support** — Click, drag, and scroll events with component handlers

---

## Examples

### Component Demo

```rust
use a3s_tui::components::{Alert, AlertKind, Badge, Table, Tabs};
use a3s_tui::{col, ElementModel, ElementProgramBuilder};

struct Demo {
    tabs: Tabs,
}

impl ElementModel for Demo {
    type Msg = Msg;

    fn view(&self) -> Element<Msg> {
        col![
            self.tabs.element(),
            Alert::new(AlertKind::Success, "All systems operational.").element(),
            Badge::new("v0.1.0").color(Color::Green).element(),
            Table::new(vec!["Name", "Status"])
                .row(vec!["Server", "Online"])
                .element(),
        ]
    }
}
```

Run `cargo run --example demo` to see all components in action.

### Chat Application

See `examples/chat.rs` for a complete chat UI with:
- Markdown rendering with syntax highlighting
- Streaming text output
- Modal dialogs
- Scrollable viewport
- Custom keybindings

### Benchmarks

Run `cargo bench --bench rendering` to measure hot rendering paths:
- display-width helpers with ANSI and CJK text
- `ActivityBlock`, `ChipStrip`, `ConnectorBlock`, `CursorLine`, `DataTable`, `DetailPanel`, `DiffView`, `GutterBlock`, `HelpPanel`, `InputBorder`, `LevelSlider`, `LogView`, `MenuPanel`, `ModeLine`, `OutputBlock`, `PromptLine`, `Scrollbar`, `SessionStatus`, `ShimmerText`, `SplitPane`, `StatusBar`, `SubagentTracker`, `Tabs`, `TaskQueue`, `TextOverlay`, `Timeline`, `WrappedPrefixBlock`, and viewport selection string rendering
- mixed markdown rendering with task lists and code blocks

### Integration Tests

Run `cargo test --test terminal_integration` to exercise the headless terminal
pipeline from Element trees through Flexbox layout, grid painting, ANSI snapshots,
resize behavior, truncation, and incremental diff changes.

---

## Architecture

### TEA Flow

```text
┌─────────────────────────────────────────┐
│  User Input (keyboard, resize, etc.)    │
└──────────────────┬──────────────────────┘
                   │
                   ▼
         ┌─────────────────┐
         │  Event → Msg    │
         └────────┬────────┘
                  │
                  ▼
         ┌─────────────────┐
         │  update(msg)    │  ← Modify state
         └────────┬────────┘
                  │
                  ▼
         ┌─────────────────┐
         │  view()         │  ← Build Element tree
         └────────┬────────┘
                  │
                  ▼
         ┌─────────────────┐
         │  Layout Engine  │  ← Taffy Flexbox
         └────────┬────────┘
                  │
                  ▼
         ┌─────────────────┐
         │  Renderer       │  ← Paint to grid
         └────────┬────────┘
                  │
                  ▼
         ┌─────────────────┐
         │  Terminal       │  ← Crossterm output
         └─────────────────┘
```

### Element Tree

Elements are the building blocks of your UI:

```rust
pub enum Element<Msg> {
    Box(BoxElement<Msg>),      // Container with Flexbox layout
    Text(TextElement),          // Styled text
    Spacer,                     // Flexible space
}
```

Use macros for concise syntax:

```rust
col![                          // Vertical column
    text!("Header").bold(),
    row![                      // Horizontal row
        text!("Left"),
        Element::Spacer,       // Push to edges
        text!("Right"),
    ],
]
```

---

## API Reference

### Core Traits

#### `ElementModel`

```rust
pub trait ElementModel: Sized + 'static {
    type Msg: From<Event> + 'static;

    fn update(&mut self, msg: Self::Msg) -> Option<Cmd<Self::Msg>>;
    fn view(&self) -> Element<Self::Msg>;
}
```

### Builders

#### `ElementProgramBuilder`

```rust
ElementProgramBuilder::new(model)
    .with_alt_screen()         // Use alternate screen buffer
    .with_fps(30)              // Target frame rate
    .with_mouse(true)          // Enable mouse events
    .run()
    .await
```

### Macros

- `col![...]` — Vertical column (FlexDirection::Column)
- `row![...]` — Horizontal row (FlexDirection::Row)
- `text!("...")` — Text element shorthand
- `spacer!()` — Flexible spacer

---

## Comparison

| Feature | a3s-tui | ratatui | cursive |
|---------|---------|---------|---------|
| Architecture | TEA | Immediate mode | Object-oriented |
| Layout | Flexbox (Taffy) | Constraints | Linear |
| Rendering | Incremental | Full redraw | Incremental |
| Async | Native (Tokio) | Manual | Callbacks |
| Markdown | Built-in | External | External |
| Components | 58 built-in | DIY | 10+ built-in |

---

## Roadmap

- [x] TEA architecture
- [x] Element tree + Flexbox layout
- [x] 58 core components
- [x] Markdown rendering
- [x] Streaming content
- [x] Keymap system
- [x] Mouse event support
- [x] Grid layout
- [x] Animation system
- [x] Theme system
- [x] Component and core unit tests
- [x] Performance benchmarks
- [x] End-to-end terminal integration tests

---

## Contributing

Contributions are welcome! Please:

1. Follow [Microsoft Rust Guidelines](https://microsoft.github.io/rust-guidelines)
2. Run `cargo fmt` and `cargo clippy` before committing
3. Add tests for new features
4. Update documentation

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

## Acknowledgments

- [Taffy](https://github.com/DioxusLabs/taffy) — Flexbox layout engine
- [Crossterm](https://github.com/crossterm-rs/crossterm) — Terminal manipulation
- [Ink](https://github.com/vadimdemedes/ink) — React-like TUI framework (inspiration)
- [Elm](https://elm-lang.org/) — The Elm Architecture pattern
