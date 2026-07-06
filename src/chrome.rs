//! Middleware-style builders for agent terminal chrome.
//!
//! `AgentChrome` keeps the active [`Theme`] at the shell boundary and returns
//! pre-themed transcript, input, and status components. It does not own app
//! state; hosts still keep selection, routing, session, and transcript state in
//! their own model.

use crate::components::{
    ActivityBlock, Chip, ChipStrip, ConnectorBlock, InputBorder, ModeLine, OutputBlock, PromptLine,
    QueuedTask, SessionStatus, StatusBar, SubagentRow, SubagentTracker, Tabs, TaskQueue,
    ToolLogRecord, ToolLogView, ToolStatusLine,
};
use crate::theme::Theme;

#[derive(Debug, Clone, Copy)]
pub struct AgentChrome<'a> {
    theme: &'a Theme,
}

impl<'a> AgentChrome<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }

    pub fn theme(&self) -> &'a Theme {
        self.theme
    }

    pub fn activity(&self, label: impl Into<String>) -> ActivityBlock {
        ActivityBlock::new(label).with_theme(self.theme)
    }

    pub fn output(&self, title: impl Into<String>) -> OutputBlock {
        OutputBlock::new(title).with_theme(self.theme)
    }

    pub fn connector(&self) -> ConnectorBlock {
        ConnectorBlock::new().with_theme(self.theme)
    }

    pub fn tool_status(&self, label: impl Into<String>) -> ToolStatusLine {
        ToolStatusLine::new(label).with_theme(self.theme)
    }

    pub fn tool_log(&self) -> ToolLogView {
        ToolLogView::new().with_theme(self.theme)
    }

    pub fn prompt(&self, prompt: impl Into<String>) -> PromptLine {
        PromptLine::new(prompt).with_theme(self.theme)
    }

    pub fn input_border(&self) -> InputBorder {
        InputBorder::new().with_theme(self.theme)
    }

    pub fn session_status(&self, cwd: impl Into<String>) -> SessionStatus {
        SessionStatus::new(cwd).with_theme(self.theme)
    }

    pub fn status_bar(&self) -> StatusBar {
        StatusBar::new().with_theme(self.theme)
    }

    pub fn tabs(&self, labels: Vec<impl Into<String>>) -> Tabs {
        Tabs::new(labels).with_theme(self.theme)
    }

    pub fn chip_strip(&self, chips: Vec<Chip>) -> ChipStrip {
        ChipStrip::new(chips).with_theme(self.theme)
    }

    pub fn mode_line(&self, mode: impl Into<String>) -> ModeLine {
        ModeLine::new(mode).with_theme(self.theme)
    }

    pub fn task_queue(&self) -> TaskQueue {
        TaskQueue::new().with_theme(self.theme)
    }

    pub fn queued_task(&self, text: impl Into<String>) -> QueuedTask {
        QueuedTask::new(text)
    }

    pub fn subagent_tracker(&self, title: impl Into<String>) -> SubagentTracker {
        SubagentTracker::new(title).with_theme(self.theme)
    }

    pub fn subagent_row(
        &self,
        agent: impl Into<String>,
        description: impl Into<String>,
    ) -> SubagentRow {
        SubagentRow::new(agent, description)
    }

    pub fn tool_record(&self, name: impl Into<String>) -> ToolLogRecord {
        ToolLogRecord::ok(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Element;
    use crate::style::Color;
    use crate::theme::{Theme, ThemeRole};

    fn sample_theme() -> Theme {
        Theme {
            primary: Color::Magenta,
            secondary: Color::Yellow,
            bg: Color::Black,
            fg: Color::BrightWhite,
            muted: Color::BrightBlack,
            border: Color::Blue,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,
            surface: Color::BrightBlack,
            highlight: Color::BrightBlue,
        }
    }

    #[test]
    fn builds_themed_transcript_components() {
        let theme = sample_theme();
        let chrome = AgentChrome::new(&theme);

        let Element::Box(activity) = chrome.activity("Running").line("ok").element::<()>() else {
            panic!("expected activity column");
        };
        let Element::Box(activity_header) = &activity.children[0] else {
            panic!("expected activity header");
        };
        let Element::Text(activity_marker) = &activity_header.children[1] else {
            panic!("expected activity marker");
        };
        assert_eq!(
            activity_marker.style.fg,
            Some(theme.color(ThemeRole::Primary))
        );

        let Element::Box(output) = chrome.output("Ran").line("ok").element::<()>() else {
            panic!("expected output column");
        };
        let Element::Text(output_header) = &output.children[0] else {
            panic!("expected output header");
        };
        assert_eq!(
            output_header.style.fg,
            Some(theme.color(ThemeRole::Primary))
        );

        let tool_status = chrome.tool_status("Running").detail("cargo test").view(40);
        assert!(tool_status.contains("\x1b[1;35m•\x1b[0m"));

        let Element::Box(connector) = chrome.connector().line("done").element::<()>() else {
            panic!("expected connector column");
        };
        let Element::Box(connector_row) = &connector.children[0] else {
            panic!("expected connector row");
        };
        let Element::Text(connector_glyph) = &connector_row.children[1] else {
            panic!("expected connector glyph");
        };
        assert_eq!(
            connector_glyph.style.fg,
            Some(theme.color(ThemeRole::Border))
        );
    }

    #[test]
    fn builds_themed_input_and_status_components() {
        let theme = sample_theme();
        let chrome = AgentChrome::new(&theme);

        let Element::Box(prompt) = chrome.prompt("❯ ").text("hello").element::<()>() else {
            panic!("expected prompt column");
        };
        let Element::Box(prompt_row) = &prompt.children[0] else {
            panic!("expected prompt row");
        };
        let Element::Text(prompt_prefix) = &prompt_row.children[1] else {
            panic!("expected prompt prefix");
        };
        assert_eq!(
            prompt_prefix.style.fg,
            Some(theme.color(ThemeRole::Primary))
        );

        let Element::Box(border) = chrome.input_border().label("high").element::<()>(24) else {
            panic!("expected border row");
        };
        assert!(border.children.iter().any(|child| {
            matches!(
                child,
                Element::Text(text) if text.style.fg == Some(theme.color(ThemeRole::Border))
            )
        }));

        let Element::Box(status) = chrome.session_status("/tmp/a3s").element::<()>() else {
            panic!("expected session status row");
        };
        let Element::Text(workspace) = &status.children[1] else {
            panic!("expected workspace segment");
        };
        assert_eq!(workspace.style.fg, Some(theme.color(ThemeRole::Primary)));

        let Element::Box(status_bar) = chrome.status_bar().left("A3S").element::<()>() else {
            panic!("expected status bar row");
        };
        assert_eq!(status_bar.style.bg, Some(theme.color(ThemeRole::Surface)));

        let Element::Box(tabs) = chrome.tabs(vec!["Chat", "Tools"]).element::<()>() else {
            panic!("expected tabs row");
        };
        let Element::Text(active_tab) = &tabs.children[0] else {
            panic!("expected active tab");
        };
        assert_eq!(active_tab.style.bg, Some(theme.color(ThemeRole::Highlight)));

        let Element::Box(chips) = chrome
            .chip_strip(vec![Chip::new("local"), Chip::new("remote")])
            .element::<()>()
        else {
            panic!("expected chip strip row");
        };
        let Element::Text(active_chip) = &chips.children[1] else {
            panic!("expected active chip");
        };
        assert_eq!(
            active_chip.style.bg,
            Some(theme.color(ThemeRole::Highlight))
        );
    }

    #[test]
    fn builds_themed_task_and_log_components() {
        let theme = sample_theme();
        let chrome = AgentChrome::new(&theme);

        let mode_line = chrome.mode_line("auto").glyph("⏵⏵").view(40);
        assert!(mode_line.contains("\x1b[1;35m⏵⏵ auto mode on\x1b[0m"));

        let Element::Box(queue) = chrome
            .task_queue()
            .running("compile")
            .queued(chrome.queued_task("docs"))
            .element::<()>()
        else {
            panic!("expected task queue column");
        };
        let Element::Text(header) = &queue.children[0] else {
            panic!("expected task queue header");
        };
        assert_eq!(header.style.fg, Some(theme.color(ThemeRole::Border)));

        let Element::Box(tracker) = chrome
            .subagent_tracker("Extract")
            .row(chrome.subagent_row("coder", "build"))
            .element::<()>()
        else {
            panic!("expected subagent tracker column");
        };
        let Element::Box(summary) = &tracker.children[0] else {
            panic!("expected tracker summary");
        };
        let Element::Text(summary_left) = &summary.children[0] else {
            panic!("expected tracker summary text");
        };
        assert_eq!(summary_left.style.fg, Some(theme.color(ThemeRole::Primary)));

        let Element::Box(tool_log) = chrome
            .tool_log()
            .title("/output")
            .record(chrome.tool_record("read").output("ok"))
            .element::<()>(3)
        else {
            panic!("expected tool log column");
        };
        let Element::Text(title) = &tool_log.children[0] else {
            panic!("expected tool log title");
        };
        assert_eq!(title.style.fg, Some(theme.color(ThemeRole::Primary)));
    }
}
