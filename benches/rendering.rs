use a3s_tui::components::{
    highlight_selection, selected_text, ActivityBlock, CellAlign, Chip, ChipStrip, ChoicePrompt,
    ConnectorBlock, CursorLine, DataColumn, DataRow, DataTable, DetailPanel, DiffView, GutterBlock,
    HelpPanel, HelpSection, InputBorder, LevelSlider, LogView, LogViewState, MenuItem, MenuPanel,
    ModeLine, OutputBlock, PreviewItem, PreviewPanel, PromptLine, QueuedTask, Scrollbar,
    SectionHeader, SessionStatus, SessionStatusChip, ShimmerText, SideNotePanel, SliderLevel,
    SplitPane, StatusBar, SubagentRow, SubagentTracker, TabSegment, TabbedMenuItem,
    TabbedMenuPanel, TabbedMenuTab, Tabs, TaskQueue, TextOverlay, TextSelection, Timeline,
    TimelineItem, ToolLogRecord, ToolLogView, TreePicker, TreePickerItem, WelcomeBanner,
    WrappedPrefixBlock,
};
use a3s_tui::markdown::Markdown;
use a3s_tui::style::{fit_visible, truncate_visible, wrap_words_compact, Color, Style};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_text_helpers(c: &mut Criterion) {
    let styled = Style::new()
        .fg(Color::Cyan)
        .bold()
        .render(&"agent output 中文测试 ".repeat(32));
    let paragraph = "A3S terminal interfaces need stable display-width wrapping for CJK, ANSI styling, and long model output. ".repeat(32);

    c.bench_function("style/fit_visible_ansi_cjk", |b| {
        b.iter(|| fit_visible(black_box(styled.as_str()), black_box(96)))
    });
    c.bench_function("style/truncate_visible_ansi_cjk", |b| {
        b.iter(|| truncate_visible(black_box(styled.as_str()), black_box(96)))
    });
    c.bench_function("style/wrap_words_compact", |b| {
        b.iter(|| wrap_words_compact(black_box(paragraph.as_str()), black_box(88)))
    });
}

fn bench_components(c: &mut Criterion) {
    let table = sample_table();
    c.bench_function("components/data_table_view", |b| {
        b.iter(|| table.view(black_box(120), black_box(30)))
    });

    let help = sample_help_panel();
    c.bench_function("components/help_panel_view", |b| {
        b.iter(|| help.view(black_box(100), black_box(32)))
    });

    let prompt = ChoicePrompt::approval("Allow shell_command to edit files?").fill_height(true);
    c.bench_function("components/choice_prompt_view", |b| {
        b.iter(|| prompt.view(black_box(72), black_box(5)))
    });

    let menu = sample_menu_panel();
    c.bench_function("components/menu_panel_view", |b| {
        b.iter(|| menu.view(black_box(88), black_box(12)))
    });

    let chip_strip = sample_chip_strip();
    c.bench_function("components/chip_strip_view", |b| {
        b.iter(|| chip_strip.view(black_box(100)))
    });

    let logs = sample_log_view();
    c.bench_function("components/log_view_view", |b| {
        b.iter(|| logs.view(black_box(100), black_box(24)))
    });

    let output = sample_output_block();
    c.bench_function("components/output_block_view", |b| {
        b.iter(|| output.view(black_box(96)))
    });

    let connector = sample_connector_block();
    c.bench_function("components/connector_block_view", |b| {
        b.iter(|| connector.view(black_box(96)))
    });

    let activity = sample_activity_block();
    c.bench_function("components/activity_block_view", |b| {
        b.iter(|| activity.view())
    });

    let prompt_line = sample_prompt_line();
    c.bench_function("components/prompt_line_view", |b| {
        b.iter(|| prompt_line.view())
    });

    let input_border = sample_input_border();
    c.bench_function("components/input_border_view", |b| {
        b.iter(|| input_border.view(black_box(120)))
    });

    let level_slider = sample_level_slider();
    c.bench_function("components/level_slider_view", |b| {
        b.iter(|| level_slider.view(black_box(120)))
    });

    let gutter = sample_gutter_block();
    c.bench_function("components/gutter_block_view", |b| b.iter(|| gutter.view()));

    let bubble = sample_gutter_bubble();
    c.bench_function("components/gutter_block_bubble_view", |b| {
        b.iter(|| bubble.view())
    });

    let wrapped = sample_wrapped_prefix_block();
    c.bench_function("components/wrapped_prefix_block_view", |b| {
        b.iter(|| wrapped.view())
    });

    let cursor_line =
        CursorLine::new("let message = format!(\"streaming transcript 中文 payload {idx}\");")
            .scroll_col(12)
            .cursor_col(36)
            .width(80)
            .cursor_style(Style::new().fg(Color::Black).bg(Color::BrightWhite))
            .fill_width(true);
    c.bench_function("components/cursor_line_view", |b| {
        b.iter(|| cursor_line.view())
    });

    let shimmer = ShimmerText::new("Working...")
        .phase(128)
        .speed_divisor(3)
        .cycle_gap(12);
    c.bench_function("components/shimmer_text_view", |b| {
        b.iter(|| shimmer.view())
    });

    let details = sample_detail_panel();
    c.bench_function("components/detail_panel_view", |b| {
        b.iter(|| details.view(black_box(100), black_box(12)))
    });

    let section_header = sample_section_header();
    c.bench_function("components/section_header_view", |b| {
        b.iter(|| section_header.view(black_box(100), black_box(4)))
    });

    let timeline = sample_timeline();
    c.bench_function("components/timeline_view", |b| {
        b.iter(|| timeline.view(black_box(52), black_box(16)))
    });

    let preview = sample_preview_panel();
    c.bench_function("components/preview_panel_view", |b| {
        b.iter(|| preview.view(black_box(88), black_box(14)))
    });

    let tree_picker = sample_tree_picker();
    c.bench_function("components/tree_picker_view", |b| {
        b.iter(|| tree_picker.view(black_box(88), black_box(14)))
    });

    let tabbed_menu = sample_tabbed_menu_panel();
    c.bench_function("components/tabbed_menu_panel_view", |b| {
        b.iter(|| tabbed_menu.view(black_box(88), black_box(14)))
    });

    let tool_log = sample_tool_log_view();
    c.bench_function("components/tool_log_view_view", |b| {
        b.iter(|| tool_log.view(black_box(100), black_box(32)))
    });

    let welcome = sample_welcome_banner();
    c.bench_function("components/welcome_banner_view", |b| {
        b.iter(|| welcome.view(black_box(100), black_box(14)))
    });

    let side_note = sample_side_note_panel();
    c.bench_function("components/side_note_panel_view", |b| {
        b.iter(|| side_note.view(black_box(88), black_box(16)))
    });

    let tabs = sample_tabs();
    c.bench_function("components/tabs_view", |b| {
        b.iter(|| tabs.view(black_box(100)))
    });

    let status = sample_status_bar();
    c.bench_function("components/status_bar_view", |b| {
        b.iter(|| status.view(black_box(100)))
    });

    let session_status = sample_session_status();
    c.bench_function("components/session_status_view", |b| {
        b.iter(|| session_status.view(black_box(120)))
    });

    let subagent_tracker = sample_subagent_tracker();
    c.bench_function("components/subagent_tracker_view", |b| {
        b.iter(|| subagent_tracker.view(black_box(120)))
    });

    let task_queue = sample_task_queue();
    c.bench_function("components/task_queue_view", |b| {
        b.iter(|| task_queue.view(black_box(120)))
    });

    let mode_line = sample_mode_line();
    c.bench_function("components/mode_line_view", |b| {
        b.iter(|| mode_line.view(black_box(120)))
    });

    let scrollbar = sample_scrollbar_view();
    let scroll_text = sample_log_text();
    c.bench_function("components/scrollbar_append_to_view", |b| {
        b.iter(|| scrollbar.append_to_view(black_box(scroll_text.as_str()), black_box(96)))
    });

    let selection_view = sample_selection_view();
    let selection = TextSelection::from_cells(12, 4, 28, 42);
    let selection_style = Style::new()
        .bg(Color::Rgb(58, 64, 88))
        .fg(Color::BrightWhite);
    c.bench_function("components/viewport_selected_text", |b| {
        b.iter(|| selected_text(black_box(selection_view.as_str()), black_box(selection)))
    });
    c.bench_function("components/viewport_highlight_selection", |b| {
        b.iter(|| {
            highlight_selection(
                black_box(selection_view.as_str()),
                black_box(selection),
                black_box(&selection_style),
            )
        })
    });

    let frame = sample_overlay_frame();
    let overlay_rows = sample_overlay_rows();
    let overlay = TextOverlay::new(overlay_rows).above_bottom(5).width(100);
    c.bench_function("components/text_overlay_apply", |b| {
        b.iter(|| overlay.apply(black_box(frame.as_str())))
    });

    let split = sample_split_pane();
    c.bench_function("components/split_pane_view", |b| {
        b.iter(|| split.view(black_box(120), black_box(28)))
    });

    let before = sample_diff_before();
    let after = sample_diff_after();
    c.bench_function("components/diff_view_from_texts_view", |b| {
        b.iter(|| {
            DiffView::from_texts(
                black_box("src/lib.rs"),
                black_box(before.as_str()),
                black_box(after.as_str()),
            )
            .view(black_box(100), black_box(32))
        })
    });
}

fn bench_markdown(c: &mut Criterion) {
    let markdown = Markdown::new().with_width(96);
    let input = r#"# Roadmap

The terminal UI renders streaming model output, task plans, tables, and code blocks.

- [x] Shared display-width helpers
- [x] Common checklist component
- [x] Common help panel component
- [ ] End-to-end terminal integration tests

```rust
pub fn render(output: &str) -> String {
    output.lines().take(8).collect::<Vec<_>>().join("\n")
}
```
"#;

    c.bench_function("markdown/render_mixed_content", |b| {
        b.iter(|| markdown.render(black_box(input)))
    });
}

fn sample_table() -> DataTable {
    let mut table = DataTable::new(vec![
        DataColumn::new("PID")
            .width(8)
            .align(CellAlign::Right)
            .priority(100),
        DataColumn::new("CPU")
            .width(8)
            .align(CellAlign::Right)
            .priority(90),
        DataColumn::new("MEM")
            .width(8)
            .align(CellAlign::Right)
            .priority(80),
        DataColumn::new("COMMAND").min_width(24).priority(70),
    ])
    .selected(Some(12))
    .scroll(4);

    for idx in 0..200 {
        let row = DataRow::new(vec![
            (1000 + idx).to_string(),
            format!("{:.1}", (idx % 97) as f32 / 3.0),
            format!("{:.1}", (idx % 61) as f32 / 2.0),
            format!("codex exec task-{idx} --workspace crates/tui 中文"),
        ])
        .cell_fg(1, Color::Green)
        .cell_fg(2, Color::Yellow);
        table.add_row(row);
    }

    table
}

fn sample_help_panel() -> HelpPanel {
    HelpPanel::new("A3S CODE - help")
        .section(
            HelpSection::new("Slash commands")
                .row("/model", "pick the model")
                .row("/ide", "file tree and code viewer")
                .row("/top", "live process monitor")
                .row("/btw <q>", "ask a background side-question"),
        )
        .section(
            HelpSection::new("Keys")
                .row("Enter", "send while idle or queue while busy")
                .row("Shift+Tab", "cycle run mode")
                .row("wheel / PgUp / PgDn", "scroll transcript")
                .row("Esc", "interrupt or close the active panel"),
        )
        .footer("Resume a past session with: a3s code resume <id>")
        .fill_height(true)
}

fn sample_menu_panel() -> MenuPanel {
    let mut panel = MenuPanel::new("Command palette")
        .subtitle("Enter run · Esc close")
        .label_width(12)
        .max_items(10)
        .selected(8)
        .footer("type to filter");

    for idx in 0..40 {
        panel.add_item(
            MenuItem::new(format!("/skill-{idx}"))
                .description(format!("run skill {idx} with contextual project data 中文"))
                .checked(idx % 3 == 0),
        );
    }

    panel
}

fn sample_chip_strip() -> ChipStrip {
    ChipStrip::new(vec![
        Chip::new("a3s-code").color(Color::Cyan),
        Chip::new("Claude Code").color(Color::Yellow),
        Chip::new("Codex").color(Color::Rgb(115, 218, 202)),
        Chip::new("OS gateway").color(Color::Magenta),
    ])
    .active(2)
}

fn sample_log_view() -> LogView {
    let lines = (0..240)
        .map(|idx| {
            format!(
                "2026-06-30T12:{:02}:00Z worker-{idx}: processed request with 中文 payload and status={}",
                idx % 60,
                if idx % 17 == 0 { "retry" } else { "ok" }
            )
        })
        .collect::<Vec<_>>();

    LogView::new("logs api-service")
        .state(LogViewState::Refreshing)
        .metadata("tail 200")
        .metadata("timestamps:on")
        .metadata("follow:on")
        .lines(lines)
        .scroll(80)
        .footer("t timestamps · f follow · r refresh")
        .fill_height(true)
}

fn sample_output_block() -> OutputBlock {
    let mut block = OutputBlock::new("Ran")
        .detail("cargo test --all-targets")
        .max_body_lines(8);
    for idx in 0..120 {
        block.add_line(format!(
            "test component_{idx} ... ok with 中文 diagnostic payload and timing {}ms",
            idx % 37
        ));
    }
    block
}

fn sample_connector_block() -> ConnectorBlock {
    let mut block = ConnectorBlock::new()
        .max_rows(6)
        .text_color(Color::BrightBlack)
        .connector_color(Color::BrightBlack);
    for idx in 0..80 {
        block.add_line(format!(
            "tool output row {idx}: compact task result with 中文 payload {}",
            idx % 13
        ));
    }
    block
}

fn sample_activity_block() -> ActivityBlock {
    let mut block = ActivityBlock::new("Running")
        .detail("cargo test --all-targets")
        .width(96)
        .marker_colors(Color::Yellow, Color::BrightBlack)
        .output_color(Color::BrightBlack);
    for idx in 0..40 {
        block.add_line(format!(
            "test live_activity_{idx} ... ok with streamed 中文 output payload {}",
            idx % 17
        ));
    }
    block
}

fn sample_prompt_line() -> PromptLine {
    PromptLine::new("❯ ")
        .text("codex exec continue extracting reusable TUI components\nwith multiline prompt input and 中文 payload")
        .margin(2)
        .width(96)
        .prompt_color(Color::Cyan)
        .text_color(Color::BrightWhite)
}

fn sample_input_border() -> InputBorder {
    InputBorder::new()
        .context("75% context used")
        .label("◇ max")
        .rule_color(Color::BrightBlack)
        .context_color(Color::BrightBlack)
        .label_color(Color::Cyan)
}

fn sample_level_slider() -> LevelSlider {
    LevelSlider::new(vec![
        SliderLevel::new("low").color(Color::Green),
        SliderLevel::new("medium").color(Color::Cyan),
        SliderLevel::new("high")
            .description("higher effort = more reasoning tokens (slower, deeper)")
            .color(Color::Yellow),
        SliderLevel::new("max").color(Color::Rgb(255, 140, 0)),
        SliderLevel::new("ultra").color(Color::Magenta),
    ])
    .title("Effort")
    .range_labels("Faster", "Smarter")
    .selected(3)
    .separator_after(3)
    .hint("←/→ adjust · Enter confirm · Esc cancel")
}

fn sample_gutter_block() -> GutterBlock {
    GutterBlock::new(
        "Rendered assistant transcript output with markdown, tool summaries, and 中文 payload.\nContinuation line with preserved alignment.",
    )
    .marker_color(Color::Green)
    .margin(2)
}

fn sample_gutter_bubble() -> GutterBlock {
    GutterBlock::new(
        "User asked for a multi-line task with 中文 content.\nSecond line stays aligned inside the bubble.",
    )
    .margin(2)
    .width(100)
    .content_color(Color::BrightWhite)
    .background_color(Color::Rgb(38, 45, 64))
}

fn sample_wrapped_prefix_block() -> WrappedPrefixBlock {
    WrappedPrefixBlock::new(
        "The model is reasoning about a long-running terminal workflow with CJK 中文 payloads and continuation rows that must line up under the thought icon.",
    )
    .margin(2)
    .width(96)
    .prefixes("💭 ", "   ")
    .style(Style::new().fg(Color::BrightBlack).italic())
}

fn sample_detail_panel() -> DetailPanel {
    DetailPanel::new("process 4242 · ppid 42 · risk high")
        .pair(
            "resources",
            "cpu 94.1% · mem 512 MiB · elapsed 00:03:42 · children 7",
        )
        .pair(
            "activity",
            "events 142 · tools 51 · sec 4 · files 23 · net 2 · llm 15 · tokens 128k",
        )
        .pair("model", "gpt-5-codex · provider openai · latency 873ms")
        .pair("cwd", "/Users/roylin/code/a3s/crates/tui")
        .pair(
            "command",
            "codex exec --workspace crates/tui continue extracting reusable detail panel 中文",
        )
        .action("o focus · / filter · ! risk · g kind · K terminate")
        .fill_height(true)
}

fn sample_section_header() -> SectionHeader {
    SectionHeader::new("agent view codex · pid 4242")
        .metadata("ppid 42 · elapsed 00:03:42 · children 7 · subtree cpu 94.1% mem 21.4%")
        .metadata("cwd /Users/roylin/code/a3s/crates/tui · 状态 healthy")
}

fn sample_timeline() -> Timeline {
    let mut timeline = Timeline::new().fill_height(true);
    for day in ["today", "yesterday", "2026-06-28"] {
        timeline = timeline.section(day);
        for idx in 0..16 {
            let color = match idx % 4 {
                0 => Color::Cyan,
                1 => Color::Yellow,
                2 => Color::Green,
                _ => Color::Magenta,
            };
            timeline = timeline.item(
                TimelineItem::new(
                    format!("{idx}m"),
                    if idx % 3 == 0 { "fact" } else { "fix" },
                    format!("memory timeline entry {idx} with 中文 payload and long preview"),
                )
                .color(color),
            );
        }
    }
    timeline.selected_item(18)
}

fn sample_preview_panel() -> PreviewPanel {
    let sample = [
        Style::new()
            .fg(Color::BrightBlack)
            .render("// syntax preview"),
        format!(
            "{} {}{}",
            Style::new().fg(Color::Magenta).render("fn"),
            Style::new().fg(Color::Cyan).render("compute"),
            "(n: usize) -> String {"
        ),
        format!(
            "    {} {}",
            Style::new().fg(Color::Magenta).render("let"),
            "total = n * 42;"
        ),
        "    format!(\"sum: {}\", total)".to_string(),
        "}".to_string(),
    ];

    PreviewPanel::new("Theme")
        .subtitle("Enter apply · Esc cancel")
        .items(vec![
            PreviewItem::new("Atom One Dark").description("default"),
            PreviewItem::new("Ayu Mirage").color(Color::Yellow),
            PreviewItem::new("Quiet Light"),
            PreviewItem::new("Nord").color(Color::Cyan),
            PreviewItem::new("Dracula").color(Color::Magenta),
        ])
        .selected(3)
        .preview_title("syntax preview")
        .preview_lines(sample.into_iter().collect::<Vec<_>>())
        .footer("↑/↓ preview · Enter apply · Esc cancel")
        .fill_height(true)
}

fn sample_tree_picker() -> TreePicker {
    let mut items = Vec::new();
    for crate_name in [
        "acl", "ahp", "box", "code", "event", "gateway", "lane", "memory", "power", "search",
        "updater",
    ] {
        items.push(TreePickerItem::branch(format!("crates/{crate_name}")).open(true));
        items.push(
            TreePickerItem::leaf("src/lib.rs")
                .depth(1)
                .description("modified"),
        );
        items.push(TreePickerItem::leaf("Cargo.toml").depth(1));
        items.push(TreePickerItem::leaf("README.md").depth(1));
    }

    TreePicker::new("@ file")
        .subtitle("↑/↓ · →/← folder · Enter · Esc")
        .items(items)
        .selected(22)
        .max_items(12)
        .footer("type to filter · 44 visible")
        .fill_height(true)
}

fn sample_tabbed_menu_panel() -> TabbedMenuPanel {
    let mut a3s_items = Vec::new();
    for idx in 0..18 {
        a3s_items.push(
            TabbedMenuItem::new(format!("openai/gpt-5-{idx}"))
                .prefix(if idx == 2 { "●" } else { " " })
                .description("configured"),
        );
    }

    let mut relay_items = Vec::new();
    for idx in 0..16 {
        relay_items.push(
            TabbedMenuItem::new(format!(
                "codex session {idx} · task handoff with 中文 payload"
            ))
            .description(format!("{}m ago", idx + 1)),
        );
    }

    TabbedMenuPanel::new(vec![
        TabbedMenuTab::new("a3s-code", Color::Cyan).items(a3s_items),
        TabbedMenuTab::new("Codex", Color::Rgb(115, 218, 202)).items(relay_items),
        TabbedMenuTab::new("Claude", Color::Yellow).empty_text("(no Claude sessions here)"),
    ])
    .title("Select model")
    .hint("↑/↓ model · ←/→ account · Enter · Esc")
    .active_tab(1)
    .selected(12)
    .max_items(10)
    .items_use_tab_color(true)
    .footer("3 sources")
    .fill_height(true)
}

fn sample_tool_log_view() -> ToolLogView {
    let mut view = ToolLogView::new()
        .title("/output")
        .max_output_lines_per_record(6)
        .fill_height(true);
    for idx in 0_i32..64 {
        let status = if idx % 9 == 0 {
            ToolLogRecord::exit("bash", 2)
        } else {
            ToolLogRecord::ok(if idx % 3 == 0 {
                "read"
            } else {
                "shell_command"
            })
        };
        view.add_record(
            status.args(format!(
                r#"{{"idx":{idx},"path":"crates/tui/src/components/tool_{idx}.rs"}}"#
            ))
            .output(format!(
                "line one for tool {idx}\nline two with 中文 payload {}\nline three with longer diagnostic output {}",
                idx % 7,
                "x".repeat(48)
            )),
        );
    }
    view.scroll(120)
}

fn sample_welcome_banner() -> WelcomeBanner {
    WelcomeBanner::new()
        .mascot_lines(vec![
            "     .-^-.      ",
            "    /_____\\     ",
            "    ( o o )     ",
            "  |  /|_|\\  _   ",
            " -+- |   | |#|  ",
            "  |  |___| \\#/  ",
            "     /   \\      ",
        ])
        .art_lines(vec![
            " █████╗ ██████╗ ███████╗     ██████╗ ██████╗ ██████╗ ███████╗",
            "██╔══██╗╚════██╗██╔════╝    ██╔════╝██╔═══██╗██╔══██╗██╔════╝",
            "███████║ █████╔╝███████╗    ██║     ██║   ██║██║  ██║█████╗",
            "██╔══██║ ╚═══██╗╚════██║    ██║     ██║   ██║██║  ██║██╔══╝",
            "██║  ██║██████╔╝███████║    ╚██████╗╚██████╔╝██████╔╝███████╗",
            "╚═╝  ╚═╝╚═════╝ ╚══════╝     ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝",
        ])
        .art_offset(1)
        .margin(2)
        .metadata("a3s-code v0.5.12 · openai/gpt-5-codex · /Users/roylin/code/a3s")
        .tip("Type a message · / for commands · Shift+Tab cycles mode · Ctrl+C twice to exit")
        .notice("a3s 0.6.0 is available · type /update to upgrade")
        .mascot_color(Color::Rgb(150, 162, 188))
        .art_color(Color::Cyan)
        .fill_height(true)
}

fn sample_side_note_panel() -> SideNotePanel {
    SideNotePanel::new("↘ by the way · Esc to close")
        .question("Can a background side-thread summarize the workspace plan while the main turn continues?")
        .answer(
            "Yes. The compact side note keeps its question and answer separate from the main transcript while preserving width-safe wrapping for 中文 payloads and long explanatory text.",
        )
        .footer("side-channel · background")
        .max_body_lines(10)
        .fill_height(true)
}

fn sample_tabs() -> Tabs {
    let mut tabs = Tabs::new(vec![
        "Agents",
        "Sessions",
        "Processes",
        "Containers",
        "Events",
    ])
    .active_colors(Color::Black, Color::Cyan)
    .inactive_color(Color::BrightBlack)
    .tab_color(0, Color::Cyan)
    .tab_color(1, Color::Yellow)
    .tab_color(2, Color::Rgb(115, 218, 202))
    .suffix("/agent")
    .segment(TabSegment::new("focus:api-service").color(Color::Cyan))
    .segment(TabSegment::new("session:codex/abc123").color(Color::Yellow));
    tabs.set_active(3);
    tabs
}

fn sample_status_bar() -> StatusBar {
    StatusBar::new()
        .left(
            " a3s top boxes:running:12 agents:42 processes:380 events:1200 high:7 llm:95 tok:128k ",
        )
        .center("q quit · / filter · Tab switch")
        .right("refreshed 0.8s ago")
        .fg(Color::BrightWhite)
        .bg(Color::Rgb(35, 40, 60))
        .bold(true)
}

fn sample_session_status() -> SessionStatus {
    SessionStatus::new("/Users/roylin/code/a3s/crates/tui")
        .branch("a3s-cli-v0.5.12")
        .model("openai/gpt-5-codex")
        .context(96_000, 128_000)
        .chip("🎯", "extract reusable components")
        .status_chip(SessionStatusChip::new("⇉", "3 agents").color(Color::Cyan))
        .status_chip(SessionStatusChip::new("⚙", "2 running").color(Color::Yellow))
}

fn sample_subagent_tracker() -> SubagentTracker {
    SubagentTracker::new("Extract reusable tui components with parallel workers")
        .slug("extract-tui")
        .row(
            SubagentRow::new("planner", "map remaining CLI panels")
                .done(true)
                .elapsed("0.8s")
                .tokens(920),
        )
        .row(
            SubagentRow::new("coder", "implement SubagentTracker")
                .elapsed("1.4s")
                .tokens(1_850),
        )
        .row(
            SubagentRow::new("reviewer", "verify terminal integration")
                .elapsed("1.2s")
                .tokens(720),
        )
}

fn sample_task_queue() -> TaskQueue {
    TaskQueue::new()
        .completed(12)
        .running("cargo test --all-targets")
        .queued(
            QueuedTask::new("update README component catalog")
                .priority(1)
                .sequence(2),
        )
        .queued(QueuedTask::new("compile benches").priority(2).sequence(3))
        .queued(
            QueuedTask::new("verify CLI dependency strategy")
                .priority(3)
                .sequence(4),
        )
}

fn sample_mode_line() -> ModeLine {
    ModeLine::new("auto")
        .glyph("⏵⏵")
        .hints("(shift+tab to cycle) · /help · ↑↓ history · esc")
        .mode_color(Color::Green)
        .hint_color(Color::BrightBlack)
}

fn sample_scrollbar_view() -> Scrollbar {
    Scrollbar::new(240, 24, 80)
        .track_color(Color::BrightBlack)
        .thumb_color(Color::Cyan)
        .hide_when_not_overflowing(true)
}

fn sample_log_text() -> String {
    (0..24)
        .map(|idx| format!("row {idx}: diagnostic output with 中文 payload"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn sample_selection_view() -> String {
    (0..40)
        .map(|idx| {
            let line = format!(
                "row {idx:02}: streaming transcript output with 中文 payload and ansi state"
            );
            if idx % 3 == 0 {
                Style::new().fg(Color::Cyan).render(&line)
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sample_overlay_frame() -> String {
    (0..80)
        .map(|idx| format!("base row {idx:02}: transcript content with 中文 payload"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn sample_overlay_rows() -> Vec<String> {
    (0..12)
        .map(|idx| {
            let raw = format!("  /command-{idx:<2} run overlay action with filtered result 中文");
            if idx == 4 {
                Style::new()
                    .fg(Color::BrightWhite)
                    .bg(Color::Cyan)
                    .render(&raw)
            } else {
                Style::new().fg(Color::BrightBlack).render(&raw)
            }
        })
        .collect()
}

fn sample_split_pane() -> SplitPane {
    let left = (0..80)
        .map(|idx| format!("{} src/module_{idx}.rs", if idx == 12 { "▸" } else { " " }))
        .collect::<Vec<_>>();
    let right = (0..120)
        .map(|idx| {
            if idx % 5 == 0 {
                format!("+ added line {idx} with 中文 content and a long explanatory suffix")
            } else {
                format!("  context line {idx} with syntax-highlight-ready content")
            }
        })
        .collect::<Vec<_>>();

    SplitPane::new(left, right)
        .title("Git")
        .subtitle("status / diff")
        .pane_titles("Files", "Diff")
        .footer("Space stage · Tab log · Esc close")
        .left_width(32)
        .fill_height(true)
}

fn sample_diff_before() -> String {
    (0..120)
        .map(|idx| {
            if idx % 9 == 0 {
                format!("let value_{idx} = compute_old({idx});")
            } else {
                format!("let value_{idx} = {idx}; // context 中文")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sample_diff_after() -> String {
    (0..120)
        .map(|idx| {
            if idx % 9 == 0 {
                format!("let value_{idx} = compute_new({idx});")
            } else if idx % 17 == 0 {
                format!("let value_{idx} = {idx}; // updated suffix 中文测试")
            } else {
                format!("let value_{idx} = {idx}; // context 中文")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

criterion_group!(
    benches,
    bench_text_helpers,
    bench_components,
    bench_markdown
);
criterion_main!(benches);
