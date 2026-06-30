use a3s_tui::components::{
    highlight_selection, selected_text, ActivityBlock, Checklist, ChecklistItem, Chip, ChipStrip,
    ChoicePrompt, ConnectorBlock, CursorLine, DetailPanel, DiffView, GitPanel, GitStatusFile,
    GutterBlock, HelpPanel, HelpSection, InputBorder, LevelSlider, LogView, LogViewState, MenuItem,
    MenuPanel, ModeLine, OutputBlock, PreviewItem, PreviewPanel, PromptLine, QueuedTask, Scrollbar,
    SessionStatus, SessionStatusChip, ShimmerText, SideNotePanel, SliderLevel, SplitPane,
    StatusBar, SubagentRow, SubagentTracker, TabSegment, TabbedMenuItem, TabbedMenuPanel,
    TabbedMenuTab, Tabs, TaskQueue, TextOverlay, TextSelection, Timeline, TimelineItem,
    ToolLogRecord, ToolLogView, TreePicker, TreePickerItem, WelcomeBanner, WrappedPrefixBlock,
};
use a3s_tui::element::{
    BorderStyle, BoxElement, Dimension, Element, FlexDirection, TextElement, TextWrap,
};
use a3s_tui::grid::Grid;
use a3s_tui::layout_engine::LayoutEngine;
use a3s_tui::paint;
use a3s_tui::style::{strip_ansi, visible_len, Color, Style};

fn render(element: &Element<()>, width: u16, height: u16) -> Grid {
    let mut engine = LayoutEngine::new();
    let layout = engine.compute(element, width, height);
    paint::paint(element, &layout, width, height)
}

fn plain(grid: &Grid) -> String {
    strip_ansi(&grid.render_to_string())
}

#[test]
fn composed_cli_like_screen_renders_through_layout_and_paint() {
    let element = cli_like_screen("A3S CODE - help");
    let grid = render(&element, 64, 34);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert_eq!(grid.width, 64);
    assert_eq!(grid.height, 34);
    assert_eq!(grid.get(0, 0).ch, '╭');
    assert_eq!(grid.get(47, 0).ch, '╮');
    assert!(plain.contains("A3S CODE - help"));
    assert!(plain.contains("collect evidence"));
    assert!(plain.contains("Allow shell_command?"));
    assert!(plain.contains("Command palette"));
    assert!(plain.contains("Workspace"));
    assert!(plain.contains("Edited src/lib.rs"));
    assert!(plain.contains("/model"));
    assert!(plain.contains("plan"));
    assert!(plain.contains("q quit"));
    assert!(ansi.contains("\x1b["));
}

#[test]
fn resize_recomputes_layout_without_leaking_past_terminal_bounds() {
    let element = cli_like_screen("A3S CODE - help");

    for (width, height) in [(40, 10), (90, 24)] {
        let grid = render(&element, width, height);
        let snapshot = grid.render_to_string();

        assert_eq!(grid.width, width);
        assert_eq!(grid.height, height);
        for line in snapshot.lines() {
            assert_eq!(
                visible_len(line),
                width as usize,
                "line should fill the terminal width: {line:?}"
            );
        }
    }
}

#[test]
fn incremental_grid_diff_tracks_only_changed_cells_between_frames() {
    let first = render(&counter_screen(1), 24, 3);
    let second = render(&counter_screen(2), 24, 3);
    let changes = first.diff(&second);

    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].x, 7);
    assert_eq!(changes[0].y, 0);
    assert_eq!(changes[0].cell.ch, '2');
}

#[test]
fn truncating_text_is_clipped_during_terminal_paint() {
    let element: Element<()> = Element::Box(
        BoxElement::new()
            .direction(FlexDirection::Column)
            .width(Dimension::Points(8.0))
            .child(Element::Text(
                TextElement::new("abcdefghijk")
                    .fg(Color::Yellow)
                    .wrap(TextWrap::Truncate),
            )),
    );

    let grid = render(&element, 8, 2);
    let plain = plain(&grid);

    assert!(plain.starts_with("abcdefgh"));
    assert!(!plain.contains("ijk"));
    assert_eq!(visible_len(plain.lines().next().unwrap()), 8);
}

#[test]
fn log_view_renders_loading_and_log_lines_through_layout_and_paint() {
    let loading: Element<()> = LogView::new("logs api")
        .state(LogViewState::Loading)
        .metadata("tail 200")
        .loading_text("loading container logs...")
        .fill_height(true)
        .element();
    let loading_grid = render(&loading, 36, 4);
    let loading_plain = plain(&loading_grid);

    assert!(loading_plain.contains("logs api"));
    assert!(loading_plain.contains("loading"));
    assert!(loading_plain.contains("loading container logs"));

    let logs: Element<()> = LogView::new("logs api")
        .metadata("follow:on")
        .lines(vec!["line one", "line two"])
        .footer("r refresh")
        .element();
    let logs_grid = render(&logs, 36, 6);
    let logs_plain = plain(&logs_grid);

    assert!(logs_plain.contains("follow:on"));
    assert!(logs_plain.contains("line one"));
    assert!(logs_plain.contains("line two"));
    assert!(logs_plain.contains("r refresh"));
}

#[test]
fn output_block_renders_status_and_tail_through_layout_and_paint() {
    let element: Element<()> = OutputBlock::new("Ran")
        .detail("cargo test")
        .text("one\ntwo\nthree\nfour")
        .max_body_lines(2)
        .element();
    let grid = render(&element, 40, 4);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert!(plain.contains("Ran"));
    assert!(plain.contains("cargo test"));
    assert!(plain.contains("⎿"));
    assert!(plain.contains("… +2 earlier lines"));
    assert!(!plain.contains("one"));
    assert!(plain.contains("three"));
    assert!(plain.contains("four"));
    assert!(ansi.contains("\x1b["));
}

#[test]
fn connector_block_renders_tool_result_rows_through_layout_and_paint() {
    let element: Element<()> = ConnectorBlock::new()
        .connector_color(Color::BrightBlack)
        .text_color(Color::BrightBlack)
        .line("Task completed · planner")
        .line("child output stored in artifact")
        .element();
    let grid = render(&element, 56, 2);
    let plain = plain(&grid);

    assert!(
        plain
            .lines()
            .next()
            .unwrap()
            .starts_with("    ⎿  Task completed"),
        "{plain:?}"
    );
    assert!(
        plain
            .lines()
            .nth(1)
            .unwrap()
            .starts_with("       child output"),
        "{plain:?}"
    );
    assert_eq!(grid.get(4, 0).ch, '⎿');
    assert_eq!(grid.get(4, 0).fg, Some(Color::BrightBlack));
}

#[test]
fn chip_strip_renders_colored_active_chip_through_layout_and_paint() {
    let element: Element<()> = ChipStrip::new(vec![
        Chip::new("a3s-code").color(Color::Cyan),
        Chip::new("Codex").color(Color::Rgb(115, 218, 202)),
    ])
    .active(1)
    .element();
    let grid = render(&element, 40, 1);
    let plain = plain(&grid);

    assert!(plain.contains("a3s-code"), "{plain:?}");
    assert!(plain.contains("Codex"), "{plain:?}");
    assert_eq!(grid.get(14, 0).ch, 'C');
    assert_eq!(grid.get(14, 0).fg, Some(Color::Black));
    assert_eq!(grid.get(14, 0).bg, Some(Color::Rgb(115, 218, 202)));
    assert!(grid.get(14, 0).bold);
}

#[test]
fn activity_block_renders_running_tool_tail_through_layout_and_paint() {
    let element: Element<()> = ActivityBlock::new("Running")
        .detail("cargo test")
        .text("one\ntwo\nthree")
        .max_output_lines(2)
        .marker_colors(Color::Yellow, Color::BrightBlack)
        .output_color(Color::BrightBlack)
        .element();
    let grid = render(&element, 40, 3);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();
    let rows = plain.lines().collect::<Vec<_>>();

    assert!(rows[0].starts_with("  • Running cargo test…"), "{plain:?}");
    assert!(rows[1].starts_with("    │ two"), "{plain:?}");
    assert!(rows[2].starts_with("    │ three"), "{plain:?}");
    assert_eq!(grid.get(2, 0).ch, '•');
    assert_eq!(grid.get(2, 0).fg, Some(Color::Yellow));
    assert!(grid.get(2, 0).bold);
    assert_eq!(grid.get(4, 1).ch, '│');
    assert_eq!(grid.get(4, 1).fg, Some(Color::BrightBlack));
    assert!(ansi.contains("\x1b[1;33m•\x1b[0m"));
}

#[test]
fn cursor_line_renders_block_cursor_through_layout_and_paint() {
    let element: Element<()> = CursorLine::new("ab你好cd")
        .scroll_col(2)
        .cursor_col(4)
        .width(8)
        .cursor_style(Style::new().fg(Color::Black).bg(Color::BrightWhite))
        .fill_width(true)
        .element();
    let grid = render(&element, 10, 1);
    let ansi = grid.render_to_string();

    assert_eq!(grid.get(0, 0).ch, '你');
    assert_eq!(grid.get(2, 0).ch, '好');
    assert_eq!(grid.get(2, 0).fg, Some(Color::Black));
    assert_eq!(grid.get(2, 0).bg, Some(Color::BrightWhite));
    assert_eq!(grid.get(4, 0).ch, 'c');
    assert_eq!(grid.get(5, 0).ch, 'd');
    assert!(ansi.contains("\x1b[30;107m好\x1b[0m"), "{ansi:?}");
}

#[test]
fn prompt_line_renders_multiline_input_through_layout_and_paint() {
    let element: Element<()> = PromptLine::new("❯ ")
        .text("run tests\nwith details")
        .margin(2)
        .prompt_color(Color::Cyan)
        .text_color(Color::BrightWhite)
        .element();
    let grid = render(&element, 32, 2);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();
    let rows = plain.lines().collect::<Vec<_>>();

    assert!(rows[0].starts_with("  ❯ run tests"), "{plain:?}");
    assert!(rows[1].starts_with("    with details"), "{plain:?}");
    assert_eq!(grid.get(2, 0).ch, '❯');
    assert_eq!(grid.get(2, 0).fg, Some(Color::Cyan));
    assert!(grid.get(2, 0).bold);
    assert_eq!(grid.get(4, 1).ch, 'w');
    assert_eq!(grid.get(4, 1).fg, Some(Color::BrightWhite));
    assert!(ansi.contains("\x1b[1;36m❯\x1b[0m"));
}

#[test]
fn input_border_renders_context_effort_line_through_layout_and_paint() {
    let element: Element<()> = InputBorder::new()
        .context("70% context used")
        .label("◇ high")
        .rule_color(Color::BrightBlack)
        .label_color(Color::Cyan)
        .element(48);
    let grid = render(&element, 48, 1);
    let plain = plain(&grid);

    assert!(plain.contains("70% context used"), "{plain:?}");
    assert!(plain.contains("◇ high"), "{plain:?}");
    assert_eq!(grid.get(2, 0).ch, '─');
    assert_eq!(grid.get(2, 0).fg, Some(Color::BrightBlack));
}

#[test]
fn level_slider_renders_effort_picker_through_layout_and_paint() {
    let element: Element<()> = LevelSlider::new(vec![
        SliderLevel::new("low").color(Color::Green),
        SliderLevel::new("medium").color(Color::Cyan),
        SliderLevel::new("high").color(Color::Yellow),
        SliderLevel::new("ultra").color(Color::Magenta),
    ])
    .title("Effort")
    .range_labels("Faster", "Smarter")
    .selected(2)
    .separator_after(2)
    .hint("←/→ adjust · Enter confirm")
    .element(60);
    let grid = render(&element, 60, 6);
    let plain = plain(&grid);

    assert!(plain.contains("Effort"), "{plain:?}");
    assert!(plain.contains("Faster"), "{plain:?}");
    assert!(plain.contains("▸ high"), "{plain:?}");
    assert_eq!(grid.get(40, 2).ch, '▲');
    assert_eq!(grid.get(40, 2).fg, Some(Color::Yellow));
    assert!(grid.get(40, 2).bold);
}

#[test]
fn timeline_renders_selected_memory_row_through_layout_and_paint() {
    let element: Element<()> = Timeline::new()
        .section("today")
        .item(TimelineItem::new("2m", "fact", "workspace uses a3s-tui").color(Color::Cyan))
        .item(TimelineItem::new("18m", "risk", "network recovered").color(Color::Yellow))
        .selected_item(1)
        .element(56, 3);
    let grid = render(&element, 56, 3);
    let plain = plain(&grid);

    assert!(plain.contains("today"), "{plain:?}");
    assert!(plain.contains("risk"), "{plain:?}");
    assert!(plain.contains("network recovered"), "{plain:?}");
    assert_eq!(grid.get(3, 2).ch, '●');
    assert_eq!(grid.get(3, 2).fg, Some(Color::Black));
    assert_eq!(grid.get(3, 2).bg, Some(Color::Yellow));
}

#[test]
fn preview_panel_renders_theme_picker_through_layout_and_paint() {
    let element: Element<()> = PreviewPanel::new("Theme")
        .subtitle("Enter apply · Esc cancel")
        .items(vec![
            PreviewItem::new("Atom One Dark").description("default"),
            PreviewItem::new("Ayu Mirage").color(Color::Yellow),
            PreviewItem::new("Quiet Light"),
        ])
        .selected(1)
        .preview_title("syntax preview")
        .preview_lines(vec![
            "// syntax preview",
            "fn compute(n: usize) -> String {",
            "    format!(\"sum: {}\", n)",
            "}",
        ])
        .element();
    let grid = render(&element, 56, 10);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert!(plain.contains("Theme"), "{plain:?}");
    assert!(plain.contains("Ayu Mirage"), "{plain:?}");
    assert!(plain.contains("syntax preview"), "{plain:?}");
    assert!(plain.contains("fn compute"), "{plain:?}");
    assert_eq!(grid.get(2, 3).ch, '▸');
    assert_eq!(grid.get(2, 3).fg, Some(Color::BrightWhite));
    assert_eq!(grid.get(2, 3).bg, Some(Color::Cyan));
    assert!(ansi.contains("\x1b["));
}

#[test]
fn tabbed_menu_panel_renders_model_picker_through_layout_and_paint() {
    let element: Element<()> = TabbedMenuPanel::new(vec![
        TabbedMenuTab::new("a3s-code", Color::Cyan).items(vec![
            TabbedMenuItem::new("openai/gpt-5").prefix("●"),
            TabbedMenuItem::new("openai/gpt-5-mini"),
        ]),
        TabbedMenuTab::new("Codex", Color::Rgb(115, 218, 202)).items(vec![
            TabbedMenuItem::new("gpt-5-codex").description("local login"),
            TabbedMenuItem::new("gpt-5-codex-fast"),
        ]),
    ])
    .title("Select model")
    .hint("↑/↓ model · ←/→ account · Enter · Esc")
    .active_tab(1)
    .selected(0)
    .selected_colors(Color::Black, Color::Cyan)
    .element();
    let grid = render(&element, 72, 7);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert!(plain.contains("Select model"), "{plain:?}");
    assert!(plain.contains("a3s-code"), "{plain:?}");
    assert!(plain.contains("Codex"), "{plain:?}");
    assert!(plain.contains("gpt-5-codex"), "{plain:?}");
    assert_eq!(grid.get(2, 3).ch, 'g');
    assert_eq!(grid.get(2, 3).fg, Some(Color::Black));
    assert_eq!(grid.get(2, 3).bg, Some(Color::Cyan));
    assert!(ansi.contains("\x1b["));
}

#[test]
fn tree_picker_renders_file_picker_through_layout_and_paint() {
    let element: Element<()> = TreePicker::new("@ file")
        .subtitle("↑/↓ · →/← folder · Enter · Esc")
        .items(vec![
            TreePickerItem::branch("src").open(true),
            TreePickerItem::leaf("main.rs").depth(1),
            TreePickerItem::leaf("lib.rs").depth(1),
            TreePickerItem::branch("tests").open(false),
            TreePickerItem::leaf("README.md"),
        ])
        .selected(2)
        .footer("5 files")
        .element();
    let grid = render(&element, 56, 8);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert!(plain.contains("@ file"), "{plain:?}");
    assert!(plain.contains("▾ src"), "{plain:?}");
    assert!(plain.contains("lib.rs"), "{plain:?}");
    assert!(plain.contains("▸ tests"), "{plain:?}");
    assert_eq!(grid.get(6, 4).ch, 'l');
    assert_eq!(grid.get(6, 4).fg, Some(Color::BrightWhite));
    assert_eq!(grid.get(6, 4).bg, Some(Color::Cyan));
    assert!(ansi.contains("\x1b["));
}

#[test]
fn tool_log_view_renders_output_history_through_layout_and_paint() {
    let element: Element<()> = ToolLogView::new()
        .title("/output")
        .record(
            ToolLogRecord::ok("read")
                .args(r#"{"file_path":"src/lib.rs"}"#)
                .output("hello\nworld"),
        )
        .record(ToolLogRecord::exit("bash", 2))
        .element(8);
    let grid = render(&element, 64, 8);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert!(plain.contains("/output"), "{plain:?}");
    assert!(plain.contains("#1 · read · ok"), "{plain:?}");
    assert!(plain.contains("args:"), "{plain:?}");
    assert!(plain.contains("    hello"), "{plain:?}");
    assert!(plain.contains("#2 · bash · exit 2"), "{plain:?}");
    assert_eq!(grid.get(0, 1).ch, '#');
    assert_eq!(grid.get(0, 1).fg, Some(Color::BrightWhite));
    assert!(grid.get(0, 1).bold);
    assert!(ansi.contains("\x1b["));
}

#[test]
fn welcome_banner_renders_start_screen_through_layout_and_paint() {
    let element: Element<()> = WelcomeBanner::new()
        .mascot_lines(vec!["  .-.  ", " (o o) ", " /|_|\\ "])
        .art_lines(vec!["A3S CODE", "TERMINAL UI"])
        .art_offset(1)
        .metadata("a3s-code v0.5.0 · gpt-5")
        .tip("Type a message · / for commands")
        .notice("a3s 0.6.0 is available")
        .mascot_color(Color::BrightBlack)
        .art_color(Color::Cyan)
        .element();
    let grid = render(&element, 64, 8);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert!(plain.contains(".-."), "{plain:?}");
    assert!(plain.contains("A3S CODE"), "{plain:?}");
    assert!(plain.contains("a3s-code v0.5.0"), "{plain:?}");
    assert!(plain.contains("0.6.0 is available"), "{plain:?}");
    assert_eq!(grid.get(2, 0).ch, ' ');
    assert_eq!(grid.get(4, 0).ch, '.');
    assert_eq!(grid.get(4, 0).fg, Some(Color::BrightBlack));
    assert!(ansi.contains("\x1b["));
}

#[test]
fn side_note_panel_renders_btw_overlay_through_layout_and_paint() {
    let element: Element<()> = SideNotePanel::new("↘ by the way · Esc to close")
        .question("Can this run in the background?")
        .answer("Yes. The side note stays compact and keeps the main transcript separate.")
        .footer("side-channel")
        .element(56);
    let grid = render(&element, 56, 6);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert!(plain.contains("by the way"), "{plain:?}");
    assert!(plain.contains("Q: Can this run"), "{plain:?}");
    assert!(plain.contains("side note stays compact"), "{plain:?}");
    assert!(plain.contains("side-channel"), "{plain:?}");
    assert_eq!(grid.get(2, 0).ch, '↘');
    assert_eq!(grid.get(2, 0).fg, Some(Color::Yellow));
    assert!(grid.get(2, 0).bold);
    assert!(ansi.contains("\x1b["));
}

#[test]
fn git_panel_renders_status_overlay_through_layout_and_paint() {
    let element: Element<()> = GitPanel::new("main")
        .files(vec![
            GitStatusFile::new('M', ' ', "src/lib.rs"),
            GitStatusFile::new('?', '?', "tests/git_panel.rs"),
        ])
        .selected_file(0)
        .log_entries(vec!["1234567 initial commit", "cafebabe add git panel"])
        .diff_lines(vec![
            "diff --git a/src/lib.rs b/src/lib.rs",
            "@@ -1,2 +1,3 @@",
            "+pub mod git_panel;",
            "-inline git renderer",
        ])
        .note("ready")
        .fill_height(true)
        .element(72, 8);
    let grid = render(&element, 72, 8);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert!(plain.contains("git · main"), "{plain:?}");
    assert!(plain.contains("Status"), "{plain:?}");
    assert!(plain.contains("Log (2)"), "{plain:?}");
    assert!(plain.contains("src/lib.rs"), "{plain:?}");
    assert!(plain.contains("pub mod git_panel"), "{plain:?}");
    assert!(plain.contains("Space/s stage"), "{plain:?}");
    assert_eq!(grid.get(2, 0).ch, 'g');
    assert_eq!(grid.get(2, 0).fg, Some(Color::Cyan));
    assert!(grid.get(2, 0).bold);
    assert!(ansi.contains("\x1b["));
}

#[test]
fn session_status_renders_agent_footer_through_layout_and_paint() {
    let element: Element<()> = SessionStatus::new("/Users/roylin/code/a3s")
        .branch("main")
        .model("openai/gpt-5")
        .context(110_000, 128_000)
        .status_chip(SessionStatusChip::new("⚙", "2 running").color(Color::Yellow))
        .element();
    let grid = render(&element, 80, 1);
    let plain = plain(&grid);

    assert!(plain.contains("a3s git:(main)"), "{plain:?}");
    assert!(plain.contains("gpt-5 (128k context)"), "{plain:?}");
    assert!(plain.contains("ctx:85%"), "{plain:?}");
    assert!(plain.contains("⚙ 2 running"), "{plain:?}");
    assert_eq!(grid.get(2, 0).ch, 'a');
    assert_eq!(grid.get(2, 0).fg, Some(Color::Cyan));
    assert!(grid.get(2, 0).bold);
}

#[test]
fn subagent_tracker_renders_parallel_rows_through_layout_and_paint() {
    let element: Element<()> = SubagentTracker::new("Extract tui")
        .slug("extract-tui")
        .row(
            SubagentRow::new("planner", "map panels")
                .done(true)
                .elapsed("0.8s"),
        )
        .row(
            SubagentRow::new("coder", "build tracker")
                .elapsed("1.2s")
                .tokens(1_500),
        )
        .element();
    let grid = render(&element, 96, 2);
    let plain = plain(&grid);

    assert!(plain.contains("extract-tui"), "{plain:?}");
    assert!(plain.contains("1 running · 1/2 done"), "{plain:?}");
    assert!(plain.contains("coder  build tracker"), "{plain:?}");
    assert_eq!(grid.get(2, 0).ch, '◯');
    assert_eq!(grid.get(2, 0).fg, Some(Color::Cyan));
    assert!(grid.get(2, 0).bold);
}

#[test]
fn task_queue_renders_pinned_queue_through_layout_and_paint() {
    let element: Element<()> = TaskQueue::new()
        .completed(1)
        .running("compile workspace")
        .queued(QueuedTask::new("write docs").sequence(2))
        .queued(QueuedTask::new("run checks").sequence(1))
        .element();
    let grid = render(&element, 64, 4);
    let plain = plain(&grid);

    assert!(plain.contains("tasks · ✓ 1 done"), "{plain:?}");
    assert!(plain.contains("compile workspace"), "{plain:?}");
    assert!(plain.contains("▱ run checks"), "{plain:?}");
    assert!(plain.contains("▱ write docs"), "{plain:?}");
    assert_eq!(grid.get(2, 1).ch, '⏳');
    assert_eq!(grid.get(2, 1).fg, Some(Color::Yellow));
    assert!(grid.get(2, 1).bold);
}

#[test]
fn mode_line_renders_mode_and_hints_through_layout_and_paint() {
    let element: Element<()> = ModeLine::new("auto")
        .glyph("⏵⏵")
        .hints("(shift+tab to cycle) · /help · esc")
        .mode_color(Color::Green)
        .element();
    let grid = render(&element, 56, 1);
    let plain = plain(&grid);

    assert!(plain.starts_with("  ⏵⏵ auto mode on"), "{plain:?}");
    assert!(plain.contains("/help"), "{plain:?}");
    assert_eq!(grid.get(2, 0).ch, '⏵');
    assert_eq!(grid.get(2, 0).fg, Some(Color::Green));
    assert!(grid.get(2, 0).bold);
}

#[test]
fn shimmer_text_renders_structured_glyph_styles_through_layout_and_paint() {
    let element: Element<()> = ShimmerText::new("Go")
        .phase(0)
        .speed_divisor(1)
        .colors(Color::Rgb(0, 0, 0), Color::Rgb(100, 0, 0))
        .element();
    let grid = render(&element, 4, 1);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert!(plain.starts_with("Go"));
    assert_eq!(grid.get(0, 0).ch, 'G');
    assert_eq!(grid.get(0, 0).fg, Some(Color::Rgb(100, 0, 0)));
    assert!(grid.get(0, 0).bold);
    assert_eq!(grid.get(1, 0).ch, 'o');
    assert!(ansi.contains("\x1b[1;38;2;100;0;0mG\x1b[0m"));
}

#[test]
fn gutter_block_renders_marker_and_alignment_through_layout_and_paint() {
    let element: Element<()> = GutterBlock::new("assistant line\nnext line")
        .marker_color(Color::Green)
        .content_color(Color::BrightWhite)
        .element();
    let grid = render(&element, 32, 2);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert!(plain.lines().next().unwrap().starts_with("  ● assistant"));
    assert!(plain.lines().nth(1).unwrap().starts_with("    next line"));
    assert_eq!(grid.get(2, 0).ch, '●');
    assert_eq!(grid.get(2, 0).fg, Some(Color::Green));
    assert!(grid.get(2, 0).bold);
    assert!(ansi.contains("\x1b[1;32m●\x1b[0m"));
}

#[test]
fn gutter_block_bubble_view_fills_requested_width() {
    let rendered = GutterBlock::new("hello\nworld")
        .margin(2)
        .width(20)
        .content_color(Color::BrightWhite)
        .background_color(Color::Rgb(38, 45, 64))
        .view();

    for row in rendered.lines() {
        assert_eq!(visible_len(row), 20);
        assert!(row.contains("48;2;38;45;64"));
    }
    assert!(strip_ansi(&rendered).contains("  ● hello"));
    assert!(strip_ansi(&rendered).contains("    world"));
}

#[test]
fn wrapped_prefix_block_renders_reasoning_rows_through_layout_and_paint() {
    let element: Element<()> = WrappedPrefixBlock::new("alpha beta gamma")
        .margin(2)
        .width(14)
        .prefixes("💭 ", "   ")
        .style(Style::new().fg(Color::BrightBlack).italic())
        .element();
    let grid = render(&element, 16, 3);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();
    let rows = plain.lines().collect::<Vec<_>>();

    assert!(rows[0].starts_with("  💭  alpha"), "{plain:?}");
    assert!(rows[1].starts_with("     beta"), "{plain:?}");
    assert!(rows[2].starts_with("     gamma"), "{plain:?}");
    assert_eq!(grid.get(2, 0).ch, '💭');
    assert_eq!(grid.get(2, 0).fg, Some(Color::BrightBlack));
    assert!(grid.get(2, 0).italic);
    assert!(ansi.contains("\x1b[3;90m💭\x1b[0m"));
}

#[test]
fn detail_panel_renders_metadata_and_actions_through_layout_and_paint() {
    let element: Element<()> = DetailPanel::new("process 42 · risk high")
        .pair("cpu", "91.4%")
        .pair("cwd", "/Users/roylin/code/a3s")
        .action("o focus · / filter · K terminate")
        .element();
    let grid = render(&element, 48, 5);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert!(plain.contains("process 42"));
    assert!(plain.contains("cpu"));
    assert!(plain.contains("91.4%"));
    assert!(plain.contains("cwd"));
    assert!(plain.contains("K terminate"));
    assert!(ansi.contains("\x1b["));
}

#[test]
fn tabs_render_active_tab_and_metadata_through_layout_and_paint() {
    let mut tabs = Tabs::new(vec!["Agents", "Containers", "Events"])
        .active_colors(Color::Black, Color::Cyan)
        .inactive_color(Color::BrightBlack)
        .tab_color(0, Color::Cyan)
        .tab_color(1, Color::Yellow)
        .tab_color(2, Color::Rgb(115, 218, 202))
        .suffix("/filter")
        .segment(TabSegment::new("focus:api").color(Color::Cyan));
    tabs.set_active(1);
    let element: Element<()> = tabs.element();
    let grid = render(&element, 64, 2);
    let plain = plain(&grid);
    let ansi = grid.render_to_string();

    assert!(plain.contains("Agents"));
    assert!(plain.contains("Containers"));
    assert!(plain.contains("/filter"));
    assert!(plain.contains("focus:api"));
    assert!(ansi.contains("\x1b["));
}

#[test]
fn status_bar_preserves_right_status_when_header_is_narrow() {
    let header = StatusBar::new()
        .left(" a3s top boxes:running:12 agents:42 processes:380 events:1200 ")
        .right("live")
        .fg(Color::BrightWhite)
        .bg(Color::Rgb(35, 40, 60))
        .bold(true)
        .view(36);
    let plain = strip_ansi(&header);

    assert_eq!(visible_len(&header), 36);
    assert!(plain.ends_with("live"));
    assert!(plain.contains('…'));
    assert!(header.contains("\x1b["));
}

#[test]
fn scrollbar_appends_styled_gutter_to_text_view() {
    let view = "alpha\n中文\nomega";
    let rendered = Scrollbar::new(30, 3, 12)
        .track_color(Color::BrightBlack)
        .thumb_color(Color::Cyan)
        .append_to_view(view, 10);
    let plain = strip_ansi(&rendered);
    let rows = plain.lines().collect::<Vec<_>>();

    assert_eq!(rows.len(), 3);
    for row in rows {
        assert_eq!(visible_len(row), 11);
    }
    assert!(rendered.contains("\x1b["));
}

#[test]
fn viewport_text_selection_copies_and_highlights_visible_rows() {
    let view = format!(
        "{}\nplain 中文 row\n{}",
        Style::new().fg(Color::Red).render("alpha beta"),
        Style::new().fg(Color::Green).render("tail row"),
    );
    let selection = TextSelection::from_cells(0, 6, 1, 8);
    let highlight = Style::new()
        .bg(Color::Rgb(58, 64, 88))
        .fg(Color::BrightWhite);

    assert_eq!(selected_text(&view, selection), "beta\nplain 中");

    let rendered = highlight_selection(&view, selection, &highlight);
    let rows = rendered.split('\n').collect::<Vec<_>>();
    assert!(!rows[0].contains("\x1b[31m"));
    assert!(rows[0].contains("alpha "));
    assert!(rows[0].contains("beta"));
    assert!(!rows[1].contains("\x1b[31m"));
    assert!(rows[2].contains("\x1b[32m"));
    assert!(rendered.contains("48;2;58;64;88"));
}

#[test]
fn text_overlay_replaces_rows_near_bottom_without_resizing_frame() {
    let base = (0..12)
        .map(|idx| format!("base row {idx:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    let overlay = TextOverlay::new([
        Style::new().fg(Color::Cyan).bold().render("  command menu"),
        Style::new()
            .fg(Color::BrightWhite)
            .bg(Color::Cyan)
            .render("  /model"),
        Style::new().fg(Color::BrightBlack).render("  ↑↓ 2/8"),
    ])
    .above_bottom(5)
    .width(16);
    let rendered = overlay.apply(&base);
    let plain = strip_ansi(&rendered);
    let rows = plain.lines().collect::<Vec<_>>();

    assert_eq!(rows.len(), 12);
    assert_eq!(rows[4].trim_end(), "  command menu");
    assert_eq!(rows[5].trim_end(), "  /model");
    assert_eq!(rows[6].trim_end(), "  ↑↓ 2/8");
    assert_eq!(rows[7], "base row 07");
    assert!(rendered.contains("\x1b["));
}

fn cli_like_screen(title: &str) -> Element<()> {
    Element::Box(
        BoxElement::new()
            .direction(FlexDirection::Column)
            .border(BorderStyle::Rounded)
            .border_color(Color::Cyan)
            .padding(1)
            .width(Dimension::Points(48.0))
            .height(Dimension::Points(32.0))
            .child(Element::Text(
                TextElement::new(title).fg(Color::Cyan).bold(),
            ))
            .child(
                Checklist::new(vec![
                    ChecklistItem::new("collect evidence").done(),
                    ChecklistItem::new("extract reusable panel").active(),
                    ChecklistItem::new("verify")
                        .status(a3s_tui::components::ChecklistStatus::Pending),
                ])
                .connector(true)
                .element(),
            )
            .child(
                ChoicePrompt::approval("Allow shell_command?")
                    .selected(1)
                    .element(),
            )
            .child(
                DiffView::from_texts("src/lib.rs", "pub fn old() {}\n", "pub fn new() {}\n")
                    .changed_backgrounds(None, None)
                    .element(),
            )
            .child(
                MenuPanel::new("Command palette")
                    .label_width(10)
                    .selected(1)
                    .item(MenuItem::new("/model").description("pick model"))
                    .item(MenuItem::new("/theme").description("preview colors"))
                    .element(),
            )
            .child(
                SplitPane::new(
                    vec!["src/main.rs", "src/lib.rs"],
                    vec!["diff --git", "+ extracted reusable pane"],
                )
                .pane_titles("Workspace", "Preview")
                .left_ratio(0.42)
                .element(),
            )
            .child(
                HelpPanel::without_title()
                    .key_width(10)
                    .indent(2)
                    .section(
                        HelpSection::new("Commands")
                            .row("/model", "pick model")
                            .row("/help", "show help"),
                    )
                    .section(
                        HelpSection::new("Keys")
                            .row("Enter", "send")
                            .row("q", "quit"),
                    )
                    .element(),
            )
            .child(
                StatusBar::new()
                    .left("default")
                    .center("plan")
                    .right("q quit")
                    .element(),
            ),
    )
}

fn counter_screen(value: u8) -> Element<()> {
    Element::Text(TextElement::new(format!("Count: {value}")))
}
