use a3s_tui::components::{
    CellAlign, DataColumn, DataRow, DataTable, HelpSection, MenuItem, MenuPanel, OutputStatus,
    TextInput, Textarea, ToolLogRecord, ToolLogStatus, TreePicker, TreePickerItem,
};
use a3s_tui::element::{BoxElement, Element, FlexDirection};
use a3s_tui::event::{Event, KeyEvent};
use a3s_tui::grid::{Cell, Grid};
use a3s_tui::layout_engine::LayoutEngine;
use a3s_tui::paint;
use a3s_tui::style::{strip_ansi, Color};
use a3s_tui::{AgentChrome, Theme};
use crossterm::event::{KeyCode, KeyModifiers};

fn render(element: &Element<()>, width: u16, height: u16) -> Grid {
    let mut engine = LayoutEngine::new();
    let layout = engine.compute(element, width, height);
    paint::paint(element, &layout, width, height)
}

fn plain(grid: &Grid) -> String {
    strip_ansi(&grid.render_to_string())
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
    }
}

fn styled_cells(grid: &Grid) -> String {
    let mut rows = Vec::new();
    for y in 0..grid.height {
        for x in 0..grid.width {
            let cell = grid.get(x, y);
            if has_style(cell) {
                rows.push(format!(
                    "{x:02},{y:02} {:?} fg={:?} bg={:?} bold={} italic={} reverse={} dim={}",
                    cell.ch, cell.fg, cell.bg, cell.bold, cell.italic, cell.reverse, cell.dim
                ));
            }
        }
    }

    if rows.is_empty() {
        "<none>".to_string()
    } else {
        rows.join("\n")
    }
}

fn has_style(cell: &Cell) -> bool {
    cell.fg.is_some()
        || cell.bg.is_some()
        || cell.bold
        || cell.italic
        || cell.underline
        || cell.reverse
        || cell.dim
        || cell.strikethrough
}

#[test]
fn text_editors_plain_snapshot() {
    let mut input = TextInput::new().with_prefix("> ");
    input.handle_event(&Event::Paste("ask\nabout\tlogs".to_string()));
    input.handle_key(&key(KeyCode::Home));

    let mut textarea = Textarea::new().with_width(32).with_height(4);
    textarea.handle_event(&Event::Paste(
        "first line\nsecond\tline\nthird line".to_string(),
    ));

    let element: Element<()> = Element::Box(
        BoxElement::new()
            .direction(FlexDirection::Column)
            .children(vec![input.element(), textarea.element()]),
    );
    let grid = render(&element, 40, 6);

    insta::assert_snapshot!("text_editors_plain", plain(&grid));
}

#[test]
fn text_input_style_snapshot() {
    let mut input = TextInput::new().with_prefix("> ");
    input.set_value("deploy service");
    input.handle_key(&key(KeyCode::Home));
    input.handle_key(&key(KeyCode::Right));

    let grid = render(&input.element::<()>(), 20, 1);

    insta::assert_snapshot!("text_input_style", styled_cells(&grid));
}

#[test]
fn menu_panel_plain_snapshot() {
    let element: Element<()> = MenuPanel::new("Command palette")
        .subtitle("workspace actions")
        .items(vec![
            MenuItem::new("Open file")
                .prefix("@")
                .description("find by path"),
            MenuItem::new("Run tests").prefix("$").suffix("⌘T"),
            MenuItem::new("Toggle dark mode").checked(true),
            MenuItem::new("Deploy").disabled(true),
        ])
        .selected(1)
        .number_shortcuts(true)
        .footer("Enter select · Esc cancel")
        .element();
    let grid = render(&element, 48, 8);

    insta::assert_snapshot!("menu_panel_plain", plain(&grid));
}

#[test]
fn tree_picker_plain_snapshot() {
    let element: Element<()> = TreePicker::new("@ file")
        .subtitle("choose a source file")
        .items(vec![
            TreePickerItem::branch("src").open(true),
            TreePickerItem::leaf("lib.rs").depth(1),
            TreePickerItem::leaf("input.rs").depth(1),
            TreePickerItem::branch("tests").open(false),
            TreePickerItem::leaf("README.md"),
        ])
        .selected(2)
        .footer("5 entries")
        .element();
    let grid = render(&element, 44, 8);

    insta::assert_snapshot!("tree_picker_plain", plain(&grid));
}

#[test]
fn data_table_plain_snapshot() {
    let element: Element<()> = DataTable::new(vec![
        DataColumn::new("Name").width(12),
        DataColumn::new("State").width(10),
        DataColumn::new("CPU").width(6).align(CellAlign::Right),
    ])
    .row(DataRow::new(vec!["gateway", "ready", "2.4%"]))
    .row(DataRow::new(vec!["worker", "busy", "81.0%"]).selected(Color::Black, Color::Cyan))
    .row(DataRow::new(vec!["search", "idle", "0.3%"]))
    .selected(Some(1))
    .element(38, 5);
    let grid = render(&element, 38, 5);

    insta::assert_snapshot!("data_table_plain", plain(&grid));
}

#[test]
fn agent_chrome_code_surfaces_plain_snapshot() {
    let theme = Theme::tokyo_night();
    let chrome = AgentChrome::new(&theme);

    let live = chrome
        .activity("Running")
        .detail("cargo test tui::render")
        .line("checking shared chrome")
        .line("snapshot ready")
        .width(64)
        .view();

    let completed = chrome
        .output("Ran")
        .detail("cargo test")
        .status(OutputStatus::Success)
        .line("35 tests passed")
        .view(64);

    let plan = chrome
        .checklist(vec![
            chrome.checklist_item("collect evidence").done(),
            chrome.checklist_item("wire AgentChrome into CLI").active(),
            chrome.checklist_item("push submodule pointer"),
        ])
        .connector(true)
        .strikethrough_done(false)
        .view(64, 4);

    let diff = chrome
        .diff_texts(
            "src/tui/ui/render.rs",
            "old line\nkeep\n",
            "new line\nkeep\n",
        )
        .max_lines(6)
        .view(64, 8);

    let help = chrome
        .help_panel("A3S Code")
        .section(
            HelpSection::new("Actions")
                .row("/output", "show tool calls")
                .row("/theme", "switch code highlighting"),
        )
        .footer("Esc close")
        .view(64, 6);

    let log = chrome
        .log_view("session log")
        .metadata("2 events")
        .line("tool started")
        .line("tool finished")
        .footer("tail")
        .view(64, 5);

    let tool_log = chrome
        .tool_log()
        .records(vec![
            ToolLogRecord::new("Ran cargo test", ToolLogStatus::Ok).output("ok"),
            ToolLogRecord::new("Edited render.rs", ToolLogStatus::Exit(1))
                .args("{\"file_path\":\"src/tui/ui/render.rs\"}")
                .output("review needed"),
        ])
        .max_output_lines_per_record(1)
        .view(64, 6);

    let surface = [live, completed, plan, diff, help, log, tool_log].join("\n\n");

    insta::assert_snapshot!("agent_chrome_code_surfaces_plain", strip_ansi(&surface));
}
