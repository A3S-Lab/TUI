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
- **Rich Components** — 60 ready-to-use components (tables, modals, help panels, text editors, etc.)

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
use a3s_tui::prelude::*;
use a3s_tui::{col, text};

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

## API Surface

For application code, prefer importing from the stable prelude:

```rust
use a3s_tui::prelude::*;
use a3s_tui::{col, row, text};
```

The prelude contains the TEA program builders, event types, layout primitives,
element types, styling types, input routing, keymaps, focus helpers, and the
`components` module. It also exports `AgentChrome`, a small middleware builder
for applying a shared theme across transcript, input, status, task, help, diff,
log, and tool-log surfaces. Lower-level modules such as `paint`, `renderer`,
`layout_engine`, and individual component modules remain public for advanced
use, but the prelude is the intended starting point for semver-stable app code.

Interactive list-like components also implement small shared state traits:

| Trait | Purpose |
| --- | --- |
| `Selectable` | Read item count, read selected index, move to first/previous/next/last item, and select an item with clamping. |
| `Scrollable` | Read and set a component scroll offset, scroll by a signed delta, and jump to top/bottom with component-owned bounds. |
| `Tabbed` | Read tab count, read active tab, and switch first/previous/next/last tab with clamping. |
| `Activatable` | Check whether a selected item can emit an action, including disabled menu rows. |

These traits do not replace component-specific message enums such as
`MenuPanelMsg` or `DataTableMsg`; they give app shells and command systems a
common way to coordinate selection, scrolling, and tab state across components.

```rust
use a3s_tui::prelude::*;

fn move_down<T: Selectable>(component: &mut T) {
    component.select_next();
}

fn maybe_run<T: Activatable>(component: &T) {
    if component.can_activate_selected() {
        // Dispatch the component-specific selected action.
    }
}
```

For app-level input orchestration, use `InputRouter` to resolve keys in a fixed
order: newest captured scope, focused component bindings, then global bindings.
This keeps blocking modals, passthrough overlays, and regular focused widgets
from each hand-rolling a different shortcut policy.

```rust
use a3s_tui::prelude::*;

const PROMPT: FocusId = 1;

#[derive(Clone)]
enum Action {
    ClosePalette,
    PromptSubmit,
    Quit,
}

let mut focus = FocusManager::new();
focus.register(PROMPT);

let mut router = InputRouter::new()
    .bind_global(KeyBinding::ctrl(KeyCode::Char('c')), Action::Quit, "Quit")
    .bind_focus(PROMPT, KeyBinding::new(KeyCode::Enter), Action::PromptSubmit, "Submit prompt")
    .bind_scope("palette", KeyBinding::new(KeyCode::Esc), Action::ClosePalette, "Close palette");

router.push_capture("palette");

if let Event::Key(key) = event {
    if let Some(routed) = router.resolve_key(&key, focus.current()) {
        // Dispatch routed.action in your update function.
    }
}
```

Theme-aware applications can use stable semantic tokens instead of storing raw
colors in app state. Built-in themes have configuration-friendly names, and
`ThemeRole` maps design-system roles to concrete `Color` and `Style` values:

```rust
use a3s_tui::prelude::*;

let theme = Theme::from_builtin_name("tokyo-night").unwrap_or_default();

let title = TextElement::new("Workspace")
    .bold()
    .fg(theme.color(ThemeRole::Primary));

let selected = theme.selection_style().render("  current row");
let panel = theme.surface_style().render("Connected");

let menu = components::MenuPanel::new("Command palette")
    .item(components::MenuItem::new("/theme"))
    .with_theme(&theme);

let table = components::DataTable::new(vec![components::DataColumn::new("Name")])
    .with_theme(&theme);

let chrome = AgentChrome::new(&theme);

let transcript = chrome
    .output("Ran command")
    .line("tests passed")
    .view(80);

let prompt = chrome
    .prompt("❯ ")
    .text("/model gpt-5")
    .view();

let footer = chrome
    .session_status("/workspace/a3s")
    .model("openai/gpt-5")
    .context(42_000, 128_000)
    .view(80);

let plan = chrome
    .checklist(vec![
        chrome.checklist_item("collect evidence").done(),
        chrome.checklist_item("verify patch").active(),
    ])
    .view(80, 4);

let diff = chrome
    .diff_texts("src/lib.rs", "old\n", "new\n")
    .view(80, 8);

for preset in Theme::builtins() {
    println!("{} -> {}", preset.name(), preset.label());
}
```

## Feature Flags

Default features preserve the full current experience:

```toml
a3s-tui = "0.1"
```

For lightweight apps that do not render markdown transcripts or highlighted
code blocks, disable defaults:

```toml
a3s-tui = { version = "0.1", default-features = false }
```

Available features:

| Feature | Default | Enables |
| --- | --- | --- |
| `markdown` | Yes | `a3s_tui::markdown::Markdown` and `a3s_tui::streaming::StreamingMarkdown` via `comrak`. |
| `syntax-highlighting` | Yes | `syntect` code highlighting for markdown code blocks. Requires `markdown`. |
| `full` | No | Convenience alias for `markdown` + `syntax-highlighting`. |

Turning off `syntax-highlighting` keeps markdown rendering available but renders
code block contents as plain text.

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
| `ActivityBlock` | In-flight activity line with styled detail and optional live output tail |
| `Alert` | Colored alerts (Success/Info/Warning/Error) with string and Element rendering |
| `Badge` | Inline status badges with string and Element rendering |
| `Breadcrumb` | Hierarchical path navigation |
| `Checklist` | Status-aware task/TODO list with configurable glyph/text color |
| `ChipStrip` | Compact colored chip strip with active chip styling |
| `ChoicePrompt` | Numbered action picker for approvals and command choices with wheel/click input plus string, line, and Element rendering |
| `Confirm` | Keyboard/mouse confirmation with inline, box, and full-screen rendering |
| `ConnectorBlock` | Connector-led compact output and continuation rows |
| `CursorLine` | Display-width-aware editor line with a block cursor |
| `DataTable` | Responsive, scrollable data table with line and Element rendering plus wheel/click row selection |
| `DetailPanel` | Compact selected-row details with metadata and actions |
| `DiffView` | Unified diff renderer for edits and tool output |
| `Divider` | Element and line-rendered separators |
| `GutterBlock` | Transcript/message block with marker gutter and optional bubble background |
| `HelpPanel` | Grouped shortcut and command help |
| `InputBorder` | Input-area border line with context, effort, and ribbon variants |
| `InlineAction` | Inline action pill with optional muted detail text |
| `KeyValue` | Labeled metadata rows with string and Element rendering |
| `LevelSlider` | Discrete level slider with tick labels, wheel/click selection, and selected marker |
| `List` | Scrollable list with selection |
| `LogView` | Scrollable log/output panel with loading and empty states |
| `MenuPanel` | Titled menu with scroll windows, click selection, wheel navigation, and checkbox toggles |
| `Meter` | Compact value meter with optional value label |
| `MetricTrend` | Metric with trend visualization |
| `ModeLine` | Current mode row with shortcut hints |
| `MultiSelect` | Multi-selection list with checkboxes |
| `OutputBlock` | Status-marked transcript/output block with tail preview and styled detail support |
| `PanelFrame` | Fixed-size titled panel frame with focus-aware borders |
| `Paragraph` | Width-aware paragraph wrapping with string and Element rendering |
| `PreviewPanel` | Selectable item list with live preview plus click and wheel handling |
| `Progress` | Progress bar |
| `PromptLine` | Prompt-prefixed input text with aligned continuation rows |
| `Scrollbar` | Scrollbar indicator with offset, percent, and append-to-view rendering |
| `Select` | Single-selection dropdown |
| `SessionStatus` | Agent/session footer row with context meter and live chips |
| `SectionHeader` | Width-safe panel heading with metadata and divider rows |
| `ShimmerText` | Animated gliding highlight for activity text |
| `SideNotePanel` | Compact side-channel question and answer panel |
| `Sparkline` | Inline trend chart |
| `SplitPane` | Two-column panel for IDE, git, memory, and detail views |
| `Spinner` | Loading animation |
| `StatusBar` | Bottom/header status bar with optional background |
| `SubagentTracker` | Parallel subagent/background work tracker |
| `Table` | Data table with headers |
| `Tabs` | Tab navigation with metadata and per-tab accents |
| `TabbedMenuPanel` | Colored tab strip with mouse switching and a scroll-aware selected list |
| `TaskQueue` | Pinned running and queued task panel |
| `TextInput` | Single-line text input |
| `TextOverlay` | Compose transient overlay rows into a rendered text frame |
| `Textarea` | Multi-line text editor with scrolling |
| `Timeline` | Sectioned timeline with colored nodes and selected-row highlighting |
| `ToolLogView` | Completed tool/command history with args and indented output |
| `ToolStatusLine` | Single-line tool status with marker, detail, and suffix |
| `Toast` / `ToastManager` | Transient notifications with string and Element rendering |
| `Tree` | Expandable tree view |
| `TreePicker` | Selectable file and hierarchy picker with click and wheel handling |
| `Modal` | Overlay dialog |
| `Viewport` | Scrollable content container with reusable text-selection helpers |
| `WelcomeBanner` | First-run mascot/art banner with metadata, tips, and notices |
| `WrappedPrefixBlock` | Wrapped callout/transcript block with aligned continuation prefix |

### Component Usage Guide

Most components are intentionally small. Build screens by composing a few
purpose-built components instead of creating one large renderer. Components
generally follow one or more of these shapes:

- **Element components** return `Element<Msg>` and participate in Flexbox layout.
- **Line components** return `String` or `Vec<String>` for transcript, overlay,
  and fixed-format rendering.
- **Interactive components** expose `handle_key` and/or `handle_mouse` helpers
  and return a small message enum such as `MenuPanelMsg`, `DataTableMsg`, or
  `ChoicePromptMsg`.

#### Navigation And Selection

| Component | Use it for | Typical usage |
| --- | --- | --- |
| `Tabs` | A compact horizontal tab row where each tab changes a view. | Keep the active tab in app state; call the tab mouse handler when capture is enabled. |
| `TabbedMenuPanel` | Account/model pickers and other tabbed command menus. | Build tabs with `TabbedMenuTab`, add tab-specific `TabbedMenuItem` rows, then handle tab clicks and row selection. |
| `MenuPanel` | Slash menus, asset pickers, plugin toggles, and short overlay menus. | Create `MenuItem` rows, set `selected` and `scroll`, render a bounded `view`, then process `MenuPanelMsg`. |
| `TreePicker` | File pickers and flattened hierarchy selection. | Build `TreePickerItem::branch` and `TreePickerItem::leaf` rows from your model; handle open, close, selected, and cancelled messages. |
| `Select` | Small single-choice controls where only a selected index matters. | Store the selected index in app state and use the returned `SelectMsg` to update it. |
| `MultiSelect` | Checkbox-like multi-selection lists. | Store checked indices separately from cursor state and toggle from `MultiSelectMsg`. |
| `List` | Simple scrollable lists without rich row metadata. | Use when labels are enough and a full `MenuPanel` would be too heavy. |
| `ChoicePrompt` | Human-in-the-loop approval prompts and numbered choices. | Build `ChoicePromptItem` rows, set the current choice, then map `ChoicePromptMsg::Selected` into the domain action. |
| `Confirm` | Yes/no confirmation before a destructive action. | Render inline, boxed, or full-screen confirmation and map `ConfirmMsg` to approve/cancel behavior. |

```rust
use a3s_tui::components::{MenuItem, MenuPanel, MenuPanelMsg};

let mut menu = MenuPanel::new("Commands")
    .items(vec![
        MenuItem::new("/model").description("Switch model"),
        MenuItem::new("/theme").description("Preview themes"),
    ])
    .selected(0)
    .max_items(8);

let rendered = menu.view(64, 10).lines().collect::<Vec<_>>();

if let Some(MenuPanelMsg::Selected(index)) = menu.handle_key(&key) {
    // Run the selected command.
}
```

#### Tables, Timelines, And Structured Data

| Component | Use it for | Typical usage |
| --- | --- | --- |
| `DataTable` | Responsive process tables, activity tables, and dense operational views. | Define `DataColumn` widths/priorities, add `DataRow` values, then use `DataTableMsg` for wheel/click row selection. |
| `Table` | Static two-dimensional output where no selection or responsive hiding is needed. | Add rows and render as a compact read-only table. |
| `Timeline` | Event streams with time, status, and selected-row highlighting. | Keep events as `TimelineItem` rows and render history, memory, or run activity chronologically. |
| `DetailPanel` | Selected item details, metadata, and short action rows. | Pair with a list/table selection; update rows when selection changes. |
| `KeyValue` | Compact metadata, runtime facts, and summary fields. | Add label/value pairs for panel sidebars and diagnostics. |
| `ToolLogView` | Completed command/tool history with arguments and output. | Append `ToolLogRecord` entries as tool calls complete. |
| `LogView` | Scrollable plain output with loading/empty states. | Use for long logs where selection is less important than browsing. |
| `Sparkline` | Inline trends for CPU, memory, tokens, latency, or rates. | Feed recent numeric samples and render inside tables or status rows. |
| `MetricTrend` | A metric value plus a small trend display. | Use in dashboards where a number and motion both matter. |
| `Meter` | Compact percentage or capacity indicator. | Use for context fill, quota, progress, or health meters. |
| `Progress` | Task progress bars and phase completion. | Store progress as a normalized value and render where the task status lives. |

```rust
use a3s_tui::components::{CellAlign, DataColumn, DataRow, DataTable};

let table = DataTable::new(vec![
    DataColumn::new("PID").width(7).align(CellAlign::Right),
    DataColumn::new("CPU%").width(6).align(CellAlign::Right),
    DataColumn::new("COMMAND").min_width(16),
])
.row(DataRow::new(vec!["4242", "12.5", "a3s code"]))
.selected(Some(0))
.scroll(0);

let view = table.view(80, 12);
```

#### Transcript, Agent, And Tool Surfaces

| Component | Use it for | Typical usage |
| --- | --- | --- |
| `GutterBlock` | Chat transcript entries with a left marker and optional bubble styling. | Render assistant/user/tool blocks with consistent gutters. |
| `PromptLine` | Prompt-prefixed user input or command text. | Keep continuation rows aligned under the prompt glyph. |
| `WrappedPrefixBlock` | Wrapped reasoning, callouts, and prefixed transcript text. | Use when every wrapped line must align under a marker. |
| `OutputBlock` | Tool output summaries with status, title, and tail preview. | Display running, completed, failed, and cancelled tool output consistently. |
| `ToolStatusLine` | One-line live tool status with marker, detail, and suffix. | Use in streaming transcripts for active tool calls. |
| `ActivityBlock` | Live activity rows with optional stdout tail. | Show long-running work without taking over the whole screen. |
| `ConnectorBlock` | Compact multi-row output with connector glyphs. | Use when a tool emits related lines that should visually hang together. |
| `SubagentTracker` | Parallel subagent or background work tracking. | Add `SubagentRow` entries for worker name, description, and status. |
| `TaskQueue` | Running task plus queued follow-up work. | Render active and queued tasks near the input or status area. |
| `Checklist` | Plans, TODOs, and multi-step workflow status. | Store each `ChecklistItem` with a `ChecklistStatus` and render the current plan. |
| `InlineAction` | Inline command/action pills such as "Open view". | Pair a visible label with muted detail text and host-side click detection. |
| `Toast` / `ToastManager` | Temporary notifications and footer flashes. | Push `Toast` values into a manager and render active ones each frame. |
| `SideNotePanel` | Side-channel question/answer panels. | Use for compact auxiliary context that should not become transcript history. |
| `WelcomeBanner` | First-run or empty-state welcome surface. | Render once at startup with tips, version metadata, and notices. |

These components are also the preferred middleware building blocks for A3S Code
TUI shells. Keep shell-owned state in the application, then compose a small
adapter that passes the current `Theme` into every transcript, input, and status
surface. `AgentChrome` also exposes themed builders for mode lines, task queues,
subagent trackers, titled or untitled help panels, logs, checklists, diffs, and
tool logs:

```rust
use a3s_tui::prelude::*;

fn render_agent_chrome(theme: &Theme, width: u16) -> String {
    let chrome = AgentChrome::new(theme);

    let status = chrome
        .status_bar()
        .left("A3S Code")
        .right("live")
        .view(width);

    let tabs = chrome
        .tabs(vec!["Chat", "Tools", "Memory"])
        .view(width);

    let output = chrome
        .output("Ran")
        .detail("cargo test")
        .line("ok")
        .view(width);

    let help = chrome
        .help_panel_without_title()
        .section(components::HelpSection::new("Keys").row("Esc", "close"))
        .view(width, 4);

    let checklist = chrome
        .checklist(vec![
            chrome.checklist_item("collect evidence").done(),
            chrome.checklist_item("verify").active(),
        ])
        .view(width, 4);

    let diff = chrome
        .diff_texts("src/lib.rs", "old\n", "new\n")
        .view(width, 4);

    let border = chrome
        .input_border()
        .context("42% context used")
        .label("◇ high")
        .view(width);

    let prompt = chrome
        .prompt("❯ ")
        .text("summarize changes")
        .view();

    [status, tabs, output, help, checklist, diff, border, prompt].join("\n")
}
```

#### Text Input, Editing, And Viewports

| Component | Use it for | Typical usage |
| --- | --- | --- |
| `TextInput` | Single-line fields. | Forward key or paste events into `TextInputMsg`; paste is sanitized to one line and word-level navigation/deletion is built in. |
| `Textarea` | Multi-line prompt boxes and editors. | Configure width, height, auto-grow, and submit behavior; paste inserts newlines without submitting and word-level navigation/deletion is built in. |
| `CursorLine` | Editor rows with a visible cursor and width-safe text. | Render the active line in a fixed-width editor panel. |
| `Viewport` | Scrollable transcript or document content. | Store the viewport state and update it from page keys or mouse wheel events. |
| `Scrollbar` | Visual scroll position on text views. | Append to a rendered view or render beside a fixed-height panel. |
| `Paragraph` | Wrapped prose with optional alignment. | Use for descriptions, help text, and detail copy that must fit a width. |
| `DiffView` | Unified diff display. | Convert edits into `DiffLine` rows and render with add/remove/context styling. |
| `Markdown` support | Rich transcript and documentation rendering. | Use the markdown renderer for CommonMark content and code highlighting. |

#### Layout, Frames, And Visual Structure

| Component | Use it for | Typical usage |
| --- | --- | --- |
| `PanelFrame` | Titled fixed-size panels with focus-aware borders. | Wrap file browsers, previews, and split panes. |
| `SplitPane` | Two-column layouts such as file tree plus editor. | Provide left and right rendered rows and let the component bound widths. |
| `Modal` | Centered overlay dialogs. | Use for blocking dialogs that should sit above the current screen. |
| `TextOverlay` | Inject overlay rows into an existing rendered frame. | Compose slash menus, pickers, and prompts above the input area. |
| `Divider` | Horizontal separators in Element or line-rendered views. | Use `divider`, `divider_line`, or width-aware variants between sections. |
| `SectionHeader` | Panel section headings with metadata and divider rows. | Use above grouped rows in detail and activity panels. |
| `StatusBar` | Header/footer bars with left, center, and right regions. | Use for screen titles, active mode, or panel hints. |
| `ModeLine` | Current mode plus shortcut hints. | Render near the footer or input area. |
| `SessionStatus` | Agent/session footer with chips and context meter. | Feed cwd, branch, model, mode, and `SessionStatusChip` values. |
| `InputBorder` | Prompt box chrome. | Render the input top/bottom border with context, effort, and ribbon variants. |
| `Breadcrumb` | Path metadata and navigation context. | Render workspace paths, config paths, or nested object locations. |
| `Badge` | Inline status labels. | Use for small state markers such as "beta", "cached", or "remote". |
| `ChipStrip` | Multiple compact tags with active styling. | Build from `Chip` values for filters, modes, or scopes. |
| `Alert` | Success, info, warning, and error messages. | Pick `AlertKind` and render as a line or Element. |
| `Spinner` | Lightweight loading indicator. | Tick on a timer and render beside running labels. |
| `ShimmerText` | Animated activity text. | Use for active phases where a spinner is too small. |
| `Tree` | Read-only hierarchical display. | Use when the hierarchy is visible but not acting as a picker. |

#### Common Composition Patterns

Use a picker overlay when the user is choosing one item and should return to the
current screen immediately:

```rust
let width: u16 = 80;
let frame = "...".repeat(24);
let rows = MenuPanel::new("Theme")
    .items(theme_items)
    .selected(selected)
    .max_items(10)
    .view(width, 12)
    .lines()
    .map(str::to_string)
    .collect::<Vec<_>>();

let frame = TextOverlay::new(rows)
    .bottom()
    .width(width as usize)
    .apply(&frame);
```

Use a full-screen panel when the content needs sustained browsing:

```rust
let width: u16 = 80;
let height: usize = 24;
let body = DataTable::new(columns)
    .selected(Some(selected))
    .scroll(scroll)
    .view(width, height.saturating_sub(1));

let screen = format!("{}\n{}", StatusBar::new().left("/top").view(width), body);
```

### Layout & Styling

- **Flexbox Layout** — `FlexDirection`, `AlignItems`, `JustifyContent`
- **Dimensions** — `Auto`, `Points(f32)`, `Percent(f32)`
- **Spacing** — `padding`, `margin`, `gap`
- **Borders** — `Single`, `Double`, `Rounded`, `Thick`
- **Colors** — 16 ANSI colors + RGB support
- **Text Styles** — bold, italic, underline, dim, strikethrough

### Advanced Features

- **Markdown Rendering** — Optional CommonMark support with feature-gated syntax highlighting
- **Streaming Content** — Real-time text streaming (perfect for LLM outputs)
- **Keymap System** — Vim-like key bindings
- **Focus Management** — Tab navigation between components
- **Input Routing** — Global, focused, and captured command scopes
- **Editor Input** — Paste-aware `TextInput`/`Textarea` with word-level editing
- **Interaction Traits** — Shared `Selectable`, `Scrollable`, and `Tabbed` state contracts
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
- `ActivityBlock`, `ChipStrip`, `ConnectorBlock`, `CursorLine`, `DataTable`, `DetailPanel`, `DiffView`, `GutterBlock`, `HelpPanel`, `InputBorder`, `LevelSlider`, `LogView`, `MenuPanel`, `ModeLine`, `OutputBlock`, `PanelFrame`, `PromptLine`, `Scrollbar`, `SectionHeader`, `SessionStatus`, `ShimmerText`, `SplitPane`, `StatusBar`, `SubagentTracker`, `Tabs`, `TaskQueue`, `TextOverlay`, `Timeline`, `ToolStatusLine`, `WrappedPrefixBlock`, and viewport selection string rendering
- mixed markdown rendering with task lists and code blocks

### Integration Tests

Run `cargo test --test terminal_integration` to exercise the headless terminal
pipeline from Element trees through Flexbox layout, grid painting, ANSI snapshots,
resize behavior, truncation, and incremental diff changes.

Run `cargo test --test render_snapshots` to compare stable golden snapshots for
core widgets such as text editors, menus, tree pickers, and data tables. Update
those fixtures intentionally with `INSTA_UPDATE=always cargo test --test render_snapshots`
after reviewing the visual change.

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
    .with_mouse_support()      // Enable mouse events
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
| Markdown | Optional built-in | External | External |
| Components | 60+ built-in | DIY | 10+ built-in |

---

## Roadmap

- [x] TEA architecture
- [x] Element tree + Flexbox layout
- [x] 60+ core components
- [x] Stable application prelude
- [x] Feature-gated markdown and syntax highlighting
- [x] Shared interaction traits for selectable, scrollable, and tabbed components
- [x] Input routing for global, focused, and captured command scopes
- [x] Markdown rendering
- [x] Streaming content
- [x] Keymap system
- [x] Mouse event support
- [x] Grid layout
- [x] Animation system
- [x] Theme system with semantic token APIs
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
