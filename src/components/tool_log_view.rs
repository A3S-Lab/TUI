use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_TOOL_LOG_OUTPUT_LINES_PER_RECORD: usize = u16::MAX as usize;

/// One completed command/tool record in a [`ToolLogView`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolLogRecord {
    name: String,
    status: ToolLogStatus,
    args: Option<String>,
    output: String,
}

impl ToolLogRecord {
    pub fn new(name: impl Into<String>, status: ToolLogStatus) -> Self {
        Self {
            name: name.into(),
            status,
            args: None,
            output: String::new(),
        }
    }

    pub fn ok(name: impl Into<String>) -> Self {
        Self::new(name, ToolLogStatus::Ok)
    }

    pub fn exit(name: impl Into<String>, code: i32) -> Self {
        Self::new(name, ToolLogStatus::Exit(code))
    }

    pub fn failed(name: impl Into<String>) -> Self {
        Self::new(name, ToolLogStatus::Failed)
    }

    pub fn args(mut self, args: impl Into<String>) -> Self {
        let args = args.into();
        if !args.is_empty() {
            self.args = Some(args);
        }
        self
    }

    pub fn output(mut self, output: impl Into<String>) -> Self {
        self.output = output.into();
        self
    }

    pub fn name_value(&self) -> &str {
        &self.name
    }

    pub fn status_value(&self) -> ToolLogStatus {
        self.status
    }

    pub fn args_value(&self) -> Option<&str> {
        self.args.as_deref()
    }

    pub fn output_value(&self) -> &str {
        &self.output
    }
}

/// Status rendered for a [`ToolLogRecord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolLogStatus {
    Ok,
    Exit(i32),
    Failed,
    Cancelled,
}

impl ToolLogStatus {
    pub fn label(self) -> String {
        match self {
            Self::Ok => "ok".to_string(),
            Self::Exit(code) => format!("exit {code}"),
            Self::Failed => "failed".to_string(),
            Self::Cancelled => "cancelled".to_string(),
        }
    }
}

/// Completed tool/command history view.
///
/// This extracts the CLI `/output` pattern: one indexed header per completed
/// tool call, optional compact args, and indented output lines.
#[derive(Debug, Clone)]
pub struct ToolLogView {
    records: Vec<ToolLogRecord>,
    title: Option<String>,
    scroll: usize,
    empty_text: String,
    fill_height: bool,
    max_output_lines_per_record: Option<usize>,
    header_color: Color,
    ok_color: Color,
    error_color: Color,
    muted_color: Color,
    output_color: Color,
    title_color: Color,
}

impl ToolLogView {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            title: None,
            scroll: 0,
            empty_text: "no tool output yet".to_string(),
            fill_height: false,
            max_output_lines_per_record: None,
            header_color: Color::BrightWhite,
            ok_color: Color::Green,
            error_color: Color::Red,
            muted_color: Color::BrightBlack,
            output_color: Color::White,
            title_color: Color::Cyan,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn record(mut self, record: ToolLogRecord) -> Self {
        self.records.push(record);
        self.clamp_scroll();
        self
    }

    pub fn records(mut self, records: Vec<ToolLogRecord>) -> Self {
        self.records = records;
        self.clamp_scroll();
        self
    }

    pub fn add_record(&mut self, record: ToolLogRecord) {
        self.records.push(record);
        self.clamp_scroll();
    }

    pub fn scroll(mut self, scroll: usize) -> Self {
        self.scroll = scroll;
        self.clamp_scroll();
        self
    }

    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty_text = text.into();
        self
    }

    pub fn fill_height(mut self, enabled: bool) -> Self {
        self.fill_height = enabled;
        self
    }

    pub fn max_output_lines_per_record(mut self, max: usize) -> Self {
        self.max_output_lines_per_record = Some(max.clamp(1, MAX_TOOL_LOG_OUTPUT_LINES_PER_RECORD));
        self.clamp_scroll();
        self
    }

    pub fn header_color(mut self, color: Color) -> Self {
        self.header_color = color;
        self
    }

    pub fn status_colors(mut self, ok: Color, error: Color) -> Self {
        self.ok_color = ok;
        self.error_color = error;
        self
    }

    pub fn muted_color(mut self, color: Color) -> Self {
        self.muted_color = color;
        self
    }

    pub fn output_color(mut self, color: Color) -> Self {
        self.output_color = color;
        self
    }

    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.header_color = theme.color(ThemeRole::Foreground);
        self.ok_color = theme.color(ThemeRole::Success);
        self.error_color = theme.color(ThemeRole::Error);
        self.muted_color = theme.color(ThemeRole::Muted);
        self.output_color = theme.color(ThemeRole::Foreground);
        self.title_color = theme.color(ThemeRole::Primary);
        self
    }

    pub fn records_value(&self) -> &[ToolLogRecord] {
        &self.records
    }

    pub fn scroll_value(&self) -> usize {
        self.normalized_scroll()
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        let mut lines = self.render_lines(width, height);
        lines.truncate(height);
        if self.fill_height {
            while lines.len() < height {
                lines.push(String::new());
            }
        }

        lines
            .into_iter()
            .map(|line| fit_visible(&line, width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self, height: usize) -> Element<Msg> {
        let mut children = Vec::new();
        if height == 0 {
            return Element::Box(BoxElement::new().direction(FlexDirection::Column));
        }

        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            children.push(Element::Text(
                TextElement::new(title).fg(self.title_color).bold(),
            ));
        }

        if self.records.is_empty() {
            if children.len() < height {
                children.push(Element::Text(
                    TextElement::new(self.empty_text.as_str())
                        .fg(self.muted_color)
                        .italic(),
                ));
            }
        } else {
            let available = height.saturating_sub(children.len());
            let rows = self.plain_rows();
            let scroll = self.normalized_scroll_for_window(rows.len(), available);
            let visible = rows.into_iter().skip(scroll).take(available);
            for row in visible {
                children.push(row.element(
                    self.header_color,
                    self.status_color(row.status()),
                    self.muted_color,
                    self.output_color,
                ));
            }
        }

        children.truncate(height);
        if self.fill_height {
            while children.len() < height {
                children.push(Element::Text(TextElement::new("")));
            }
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_lines(&self, width: usize, height: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            lines.push(
                Style::new()
                    .fg(self.title_color)
                    .bold()
                    .render(&fit_visible(title, width)),
            );
        }

        if self.records.is_empty() {
            if lines.len() < height {
                lines.push(
                    Style::new()
                        .fg(self.muted_color)
                        .italic()
                        .render(&fit_visible(&self.empty_text, width)),
                );
            }
            return lines;
        }

        let available = height.saturating_sub(lines.len());
        let rows = self.plain_rows();
        let scroll = self.normalized_scroll_for_window(rows.len(), available);
        for row in rows.into_iter().skip(scroll).take(available) {
            lines.push(row.render(
                width,
                self.header_color,
                self.status_color(row.status()),
                self.muted_color,
                self.output_color,
            ));
        }
        lines
    }

    fn plain_rows(&self) -> Vec<ToolLogRow> {
        let mut rows = Vec::new();
        for (index, record) in self.records.iter().enumerate() {
            if index > 0 {
                rows.push(ToolLogRow::Blank);
            }
            rows.push(ToolLogRow::Header {
                index: index + 1,
                name: record.name.clone(),
                status: record.status,
            });
            if let Some(args) = record.args.as_deref().filter(|args| !args.is_empty()) {
                rows.push(ToolLogRow::Args(args.to_string()));
            }

            let output_lines = record.output.trim_end().lines().collect::<Vec<_>>();
            if !output_lines.is_empty() {
                rows.push(ToolLogRow::OutputLabel);
                let total = output_lines.len();
                let shown = self
                    .max_output_lines_per_record
                    .map(|max| total.min(max))
                    .unwrap_or(total);
                if shown < total {
                    rows.push(ToolLogRow::Omitted(total - shown));
                }
                for line in output_lines.iter().skip(total.saturating_sub(shown)) {
                    rows.push(ToolLogRow::Output((*line).to_string()));
                }
            }
        }
        rows
    }

    fn normalized_scroll(&self) -> usize {
        self.normalized_scroll_for_len(self.plain_rows().len())
    }

    fn normalized_scroll_for_len(&self, row_count: usize) -> usize {
        self.scroll.min(row_count.saturating_sub(1))
    }

    fn normalized_scroll_for_window(&self, row_count: usize, window_height: usize) -> usize {
        if row_count == 0 || window_height == 0 {
            return 0;
        }

        self.scroll
            .min(row_count.saturating_sub(window_height.min(row_count)))
    }

    fn clamp_scroll(&mut self) {
        self.scroll = self.normalized_scroll();
    }

    fn status_color(&self, status: ToolLogStatus) -> Color {
        match status {
            ToolLogStatus::Ok => self.ok_color,
            ToolLogStatus::Exit(_) | ToolLogStatus::Failed | ToolLogStatus::Cancelled => {
                self.error_color
            }
        }
    }
}

impl Default for ToolLogView {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
enum ToolLogRow {
    Blank,
    Header {
        index: usize,
        name: String,
        status: ToolLogStatus,
    },
    Args(String),
    OutputLabel,
    Omitted(usize),
    Output(String),
}

impl ToolLogRow {
    fn render(
        &self,
        width: usize,
        header_color: Color,
        status_color: Color,
        muted_color: Color,
        output_color: Color,
    ) -> String {
        match self {
            Self::Blank => String::new(),
            Self::Header {
                index,
                name,
                status,
            } => {
                let prefix = Style::new()
                    .fg(header_color)
                    .bold()
                    .render(&format!("#{index} · "));
                let name = Style::new()
                    .fg(header_color)
                    .bold()
                    .render(&truncate_visible(name, width.saturating_sub(8)));
                let status = Style::new()
                    .fg(status_color)
                    .bold()
                    .render(&format!(" · {}", status.label()));
                format!("{prefix}{name}{status}")
            }
            Self::Args(args) => Style::new().fg(muted_color).render(&format!(
                "  args: {}",
                truncate_visible(args, width.saturating_sub(8))
            )),
            Self::OutputLabel => Style::new().fg(muted_color).render("  output:"),
            Self::Omitted(count) => Style::new()
                .fg(muted_color)
                .render(&format!("    ... +{count} earlier lines")),
            Self::Output(line) => Style::new().fg(output_color).render(&format!(
                "    {}",
                truncate_visible(line, width.saturating_sub(4))
            )),
        }
    }

    fn element<Msg>(
        &self,
        header_color: Color,
        status_color: Color,
        muted_color: Color,
        output_color: Color,
    ) -> Element<Msg> {
        match self {
            Self::Blank => Element::Text(TextElement::new("")),
            Self::Header {
                index,
                name,
                status,
            } => Element::Box(
                BoxElement::new()
                    .direction(FlexDirection::Row)
                    .child(Element::Text(
                        TextElement::new(format!("#{index} · "))
                            .fg(header_color)
                            .bold(),
                    ))
                    .child(Element::Text(
                        TextElement::new(name.as_str()).fg(header_color).bold(),
                    ))
                    .child(Element::Text(
                        TextElement::new(format!(" · {}", status.label()))
                            .fg(status_color)
                            .bold(),
                    )),
            ),
            Self::Args(args) => {
                Element::Text(TextElement::new(format!("  args: {args}")).fg(muted_color))
            }
            Self::OutputLabel => Element::Text(TextElement::new("  output:").fg(muted_color)),
            Self::Omitted(count) => Element::Text(
                TextElement::new(format!("    ... +{count} earlier lines")).fg(muted_color),
            ),
            Self::Output(line) => {
                Element::Text(TextElement::new(format!("    {line}")).fg(output_color))
            }
        }
    }

    fn status(&self) -> ToolLogStatus {
        match self {
            Self::Header { status, .. } => *status,
            Self::Blank
            | Self::Args(_)
            | Self::OutputLabel
            | Self::Omitted(_)
            | Self::Output(_) => ToolLogStatus::Ok,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    fn sample() -> ToolLogView {
        ToolLogView::new()
            .title("/output")
            .record(
                ToolLogRecord::ok("read")
                    .args(r#"{"file_path":"/x"}"#)
                    .output("hello\nworld\n"),
            )
            .record(ToolLogRecord::exit("bash", 2))
    }

    #[test]
    fn renders_headers_args_and_indented_output() {
        let rendered = sample().view(56, 10);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("/output"));
        assert!(plain.contains("#1 · read · ok"));
        assert!(plain.contains("args: {\"file_path\":\"/x\"}"));
        assert!(plain.contains("    hello"));
        assert!(plain.contains("#2 · bash · exit 2"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 56, "{line:?}");
        }
    }

    #[test]
    fn empty_state_renders_title_and_message() {
        let rendered = ToolLogView::new()
            .title("/output")
            .empty_text("nothing yet")
            .view(32, 4);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("/output"));
        assert!(plain.contains("nothing yet"));
    }

    #[test]
    fn scrolls_flattened_record_rows() {
        let rendered = sample().scroll(3).view(40, 3);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("hello"));
        assert!(!plain.contains("#1 · read"));
    }

    #[test]
    fn scroll_past_rows_clamps_to_visible_tail_window() {
        let rendered = sample().scroll(usize::MAX).view(40, 3);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("#2 · bash · exit 2"));
        assert_eq!(rendered.lines().count(), 3);
    }

    #[test]
    fn records_reclamp_stale_scroll_after_rows_shrink() {
        let rendered = sample()
            .scroll(usize::MAX)
            .records(vec![ToolLogRecord::ok("only")])
            .view(40, 2);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("#1 · only · ok"));
    }

    #[test]
    fn normalizes_stale_scroll_when_rendering() {
        let mut view = sample();
        let last_row = view.plain_rows().len().saturating_sub(1);
        view.scroll = usize::MAX;

        assert_eq!(view.scroll_value(), last_row);

        let rendered = view.view(40, 3);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("#2 · bash · exit 2"));
        assert!(!plain.contains("#1 · read"));
        assert_eq!(rendered.lines().count(), 3);
    }

    #[test]
    fn element_normalizes_stale_scroll_when_rendering() {
        let mut view = sample();
        view.scroll = usize::MAX;

        let element = view.element::<()>(2);
        let text = element_text(&element);

        assert!(text.contains("#2 · bash · exit 2"));
        assert!(!text.contains("#1 · read"));
        match element {
            Element::Box(column) => assert_eq!(column.children.len(), 2),
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn limits_output_tail_per_record() {
        let rendered = ToolLogView::new()
            .record(ToolLogRecord::ok("test").output("one\ntwo\nthree\nfour"))
            .max_output_lines_per_record(2)
            .view(40, 6);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("+2 earlier lines"));
        assert!(!plain.contains("one"));
        assert!(plain.contains("three"));
        assert!(plain.contains("four"));
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let view = ToolLogView::new().with_theme(&theme);

        assert_eq!(view.header_color, theme.color(ThemeRole::Foreground));
        assert_eq!(view.ok_color, theme.color(ThemeRole::Success));
        assert_eq!(view.error_color, theme.color(ThemeRole::Error));
        assert_eq!(view.muted_color, theme.color(ThemeRole::Muted));
        assert_eq!(view.output_color, theme.color(ThemeRole::Foreground));
        assert_eq!(view.title_color, theme.color(ThemeRole::Primary));
    }

    #[test]
    fn cjk_output_fits_requested_width() {
        let rendered = ToolLogView::new()
            .record(
                ToolLogRecord::ok("read")
                    .args(r#"{"path":"中文路径"}"#)
                    .output("中文输出 with a long diagnostic suffix"),
            )
            .view(24, 6);

        assert!(strip_ansi(&rendered).contains("中文"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 24, "{line:?}");
        }
    }

    #[test]
    fn fill_height_pads_remaining_rows() {
        let rendered = ToolLogView::new()
            .record(ToolLogRecord::failed("patch"))
            .fill_height(true)
            .view(32, 5);

        assert_eq!(rendered.lines().count(), 5);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 32, "{line:?}");
        }
    }

    #[test]
    fn oversized_output_line_limit_is_clamped() {
        let view = ToolLogView::new()
            .record(ToolLogRecord::ok("test").output("one\ntwo"))
            .max_output_lines_per_record(usize::MAX);
        let rendered = view.view(24, 4);

        assert_eq!(
            view.max_output_lines_per_record,
            Some(MAX_TOOL_LOG_OUTPUT_LINES_PER_RECORD)
        );
        assert!(rendered.lines().all(|line| visible_len(line) == 24));
        assert_eq!(view.plain_rows().len(), 4);
    }

    #[test]
    fn zero_size_renders_empty_string() {
        assert_eq!(sample().view(0, 4), "");
        assert_eq!(sample().view(40, 0), "");
    }

    #[test]
    fn element_produces_column() {
        let el: Element<()> = sample().element(8);

        match el {
            Element::Box(column) => {
                assert_eq!(column.style.flex_direction, FlexDirection::Column);
                assert!(!column.children.is_empty());
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn element_zero_height_returns_empty_column() {
        let el: Element<()> = sample().element(0);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        assert!(column.children.is_empty());
    }

    #[test]
    fn element_title_counts_against_height_budget() {
        let el: Element<()> = sample().element(2);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        let text = column
            .children
            .iter()
            .map(element_text)
            .collect::<Vec<_>>()
            .join("");

        assert_eq!(column.children.len(), 2);
        assert!(text.contains("/output"), "{text:?}");
        assert!(text.contains("#1 · read · ok"), "{text:?}");
        assert!(!text.contains("args:"), "{text:?}");
    }

    #[test]
    fn element_empty_state_respects_title_height_budget() {
        let el: Element<()> = ToolLogView::new()
            .title("/output")
            .empty_text("nothing yet")
            .element(1);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        let text = column
            .children
            .iter()
            .map(element_text)
            .collect::<Vec<_>>()
            .join("");

        assert_eq!(column.children.len(), 1);
        assert!(text.contains("/output"), "{text:?}");
        assert!(!text.contains("nothing yet"), "{text:?}");
    }

    #[test]
    fn element_fill_height_pads_empty_rows() {
        let el: Element<()> = ToolLogView::new()
            .record(ToolLogRecord::failed("patch"))
            .fill_height(true)
            .element(4);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        let rows = column.children.iter().map(element_text).collect::<Vec<_>>();

        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0], "#1 · patch · failed");
        assert_eq!(rows[1], "");
        assert_eq!(rows[2], "");
        assert_eq!(rows[3], "");
    }

    fn element_text(element: &Element<()>) -> String {
        match element {
            Element::Text(text) => text.content.clone(),
            Element::Box(box_element) => box_element
                .children
                .iter()
                .map(element_text)
                .collect::<Vec<_>>()
                .join(""),
            Element::Spacer | Element::_Phantom(_) => String::new(),
        }
    }
}
