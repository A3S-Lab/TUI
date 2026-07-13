use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_SUBAGENT_CHILD_INDENT: usize = u16::MAX as usize;
const MAX_SUBAGENT_MARGIN: usize = u16::MAX as usize;
const MAX_SUBAGENT_RUNNING_ROWS: usize = u16::MAX as usize;

/// Durable status rows for parallel subagents or background workers.
///
/// This extracts the common CLI pattern of a summary row with aggregate
/// progress plus live child rows for workers that are still running.
#[derive(Debug, Clone)]
pub struct SubagentTracker {
    title: String,
    slug: Option<String>,
    show_slug: bool,
    rows: Vec<SubagentRow>,
    max_running_rows: usize,
    margin: usize,
    child_indent: usize,
    marker: String,
    accent_color: Color,
    active_color: Color,
    muted_color: Color,
    error_color: Color,
}

impl SubagentTracker {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            slug: None,
            show_slug: true,
            rows: Vec::new(),
            max_running_rows: 4,
            margin: 2,
            child_indent: 5,
            marker: "◯".to_string(),
            accent_color: Color::Cyan,
            active_color: Color::Magenta,
            muted_color: Color::BrightBlack,
            error_color: Color::Red,
        }
    }

    pub fn slug(mut self, slug: impl Into<String>) -> Self {
        let slug = slug.into();
        if !slug.trim().is_empty() {
            self.slug = Some(slug);
        }
        self
    }

    /// Control whether the machine-oriented slug is shown beside the title.
    /// Interactive agent panes normally prefer the user-facing title only;
    /// durable workflow views can keep the slug for correlation.
    pub fn show_slug(mut self, show: bool) -> Self {
        self.show_slug = show;
        self
    }

    pub fn row(mut self, row: SubagentRow) -> Self {
        self.rows.push(row);
        self
    }

    pub fn rows(mut self, rows: Vec<SubagentRow>) -> Self {
        self.rows = rows;
        self
    }

    pub fn add_row(&mut self, row: SubagentRow) {
        self.rows.push(row);
    }

    pub fn max_running_rows(mut self, max_running_rows: usize) -> Self {
        self.max_running_rows = max_running_rows.clamp(1, MAX_SUBAGENT_RUNNING_ROWS);
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin.min(MAX_SUBAGENT_MARGIN);
        self
    }

    pub fn child_indent(mut self, child_indent: usize) -> Self {
        self.child_indent = child_indent.min(MAX_SUBAGENT_CHILD_INDENT);
        self
    }

    pub fn marker(mut self, marker: impl Into<String>) -> Self {
        let marker = marker.into();
        if !marker.is_empty() {
            self.marker = marker;
        }
        self
    }

    pub fn accent_color(mut self, color: Color) -> Self {
        self.accent_color = color;
        self
    }

    pub fn active_color(mut self, color: Color) -> Self {
        self.active_color = color;
        self
    }

    pub fn muted_color(mut self, color: Color) -> Self {
        self.muted_color = color;
        self
    }

    pub fn error_color(mut self, color: Color) -> Self {
        self.error_color = color;
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.accent_color = theme.color(ThemeRole::Primary);
        self.active_color = theme.color(ThemeRole::Secondary);
        self.muted_color = theme.color(ThemeRole::Muted);
        self.error_color = theme.color(ThemeRole::Error);
        self
    }

    pub fn title_value(&self) -> &str {
        &self.title
    }

    pub fn slug_value(&self) -> Option<&str> {
        self.slug.as_deref()
    }

    pub fn rows_value(&self) -> &[SubagentRow] {
        &self.rows
    }

    pub fn view(&self, width: u16) -> String {
        let width = width as usize;
        if width == 0 || self.rows.is_empty() {
            return String::new();
        }

        self.render_lines(width)
            .into_iter()
            .map(|line| fit_visible(&line, width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        if self.rows.is_empty() {
            return Element::Box(BoxElement::new().direction(FlexDirection::Column));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(self.element_rows()),
        )
    }

    fn render_lines(&self, width: usize) -> Vec<String> {
        let mut lines = vec![self.render_summary(width)];
        lines.extend(
            self.running_rows()
                .into_iter()
                .map(|row| self.render_child(row, width)),
        );
        if let Some(hidden) = self.hidden_running_rows() {
            lines.push(self.render_hidden_running(hidden, width));
        }
        lines
    }

    fn render_summary(&self, width: usize) -> String {
        let right = self.summary_status();
        let left = self.summary_left(width, &right);
        self.render_aligned(left, self.accent_color, true, right, width)
    }

    fn render_child(&self, row: &SubagentRow, width: usize) -> String {
        let right = row.metadata();
        let left = self.child_left(row, width, &right);
        self.render_aligned(
            left,
            row_color(row, self.active_color, self.muted_color, self.error_color),
            false,
            right,
            width,
        )
    }

    fn render_hidden_running(&self, hidden: usize, width: usize) -> String {
        let label = format!(
            "{}… +{hidden} more running",
            " ".repeat(self.child_indent_for_width(width))
        );
        Style::new().fg(self.muted_color).render(&label)
    }

    fn render_aligned(
        &self,
        left: String,
        left_color: Color,
        bold_left: bool,
        right: String,
        width: usize,
    ) -> String {
        let content_width = [visible_len(&left), visible_len(&right), 1]
            .into_iter()
            .fold(0usize, usize::saturating_add);
        let pad = width.saturating_sub(content_width);
        let mut left_style = Style::new().fg(left_color);
        if bold_left {
            left_style = left_style.bold();
        }
        format!(
            "{}{}{}",
            left_style.render(&left),
            " ".repeat(pad),
            Style::new().fg(self.muted_color).render(&right)
        )
    }

    fn summary_left(&self, width: usize, right: &str) -> String {
        let label = if self.show_slug {
            format!("{}  {}", self.slug_text(), self.title.trim())
        } else {
            self.title.trim().to_string()
        };
        let raw = format!(
            "{}{} {}",
            " ".repeat(self.margin_for_width(width)),
            self.marker,
            label
        );
        fit_left(&raw, width, right)
    }

    fn child_left(&self, row: &SubagentRow, width: usize, right: &str) -> String {
        let raw = format!(
            "{}{} {}  {}",
            " ".repeat(self.child_indent_for_width(width)),
            self.marker,
            row.agent,
            row.description
        );
        fit_left(&raw, width, right)
    }

    fn summary_status(&self) -> String {
        let total = self.rows.len();
        let done = self.rows.iter().filter(|row| row.is_done()).count();
        let running = total.saturating_sub(done);
        let tokens = self
            .rows
            .iter()
            .map(|row| row.tokens)
            .fold(0u64, u64::saturating_add);
        let elapsed = aggregate_elapsed(&self.rows);
        let status = if done == total {
            format!("{done}/{total} agents done")
        } else {
            format!("{running} running · {done}/{total} done")
        };

        if tokens > 0 {
            format!("{status} · {elapsed} · ↓ {} tokens", fmt_tokens(tokens))
        } else {
            format!("{status} · {elapsed}")
        }
    }

    fn running_rows(&self) -> Vec<&SubagentRow> {
        self.rows
            .iter()
            .filter(|row| row.status == SubagentRowStatus::Running)
            .take(self.max_running_rows)
            .collect()
    }

    fn hidden_running_rows(&self) -> Option<usize> {
        let running = self
            .rows
            .iter()
            .filter(|row| row.status == SubagentRowStatus::Running)
            .count();
        let hidden = running.saturating_sub(self.max_running_rows);
        (hidden > 0).then_some(hidden)
    }

    fn element_rows<Msg>(&self) -> Vec<Element<Msg>> {
        let mut rows = vec![self.summary_element()];
        rows.extend(
            self.running_rows()
                .into_iter()
                .map(|row| self.child_element(row)),
        );
        if let Some(hidden) = self.hidden_running_rows() {
            rows.push(Element::Text(
                TextElement::new(format!(
                    "{}… +{hidden} more running",
                    " ".repeat(self.child_indent_for_element())
                ))
                .fg(self.muted_color),
            ));
        }
        rows
    }

    fn summary_element<Msg>(&self) -> Element<Msg> {
        let right = self.summary_status();
        let label = if self.show_slug {
            format!("{}  {}", self.slug_text(), self.title.trim())
        } else {
            self.title.trim().to_string()
        };
        let left = format!(
            "{}{} {}",
            " ".repeat(self.margin_for_element()),
            self.marker,
            label
        );
        row_element(left, self.accent_color, true, right, self.muted_color)
    }

    fn child_element<Msg>(&self, row: &SubagentRow) -> Element<Msg> {
        let left = format!(
            "{}{} {}  {}",
            " ".repeat(self.child_indent_for_element()),
            self.marker,
            row.agent,
            row.description
        );
        row_element(
            left,
            row_color(row, self.active_color, self.muted_color, self.error_color),
            false,
            row.metadata(),
            self.muted_color,
        )
    }

    fn slug_text(&self) -> String {
        self.slug
            .as_deref()
            .map(str::trim)
            .filter(|slug| !slug.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| slugify(&self.title))
    }

    fn margin_for_width(&self, width: usize) -> usize {
        self.margin.min(width).min(MAX_SUBAGENT_MARGIN)
    }

    fn child_indent_for_width(&self, width: usize) -> usize {
        self.child_indent.min(width).min(MAX_SUBAGENT_CHILD_INDENT)
    }

    fn margin_for_element(&self) -> usize {
        self.margin.min(MAX_SUBAGENT_MARGIN)
    }

    fn child_indent_for_element(&self) -> usize {
        self.child_indent.min(MAX_SUBAGENT_CHILD_INDENT)
    }
}

/// Lifecycle state rendered for one parallel subagent row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubagentRowStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
    TrackingLost,
}

impl SubagentRowStatus {
    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::Running)
    }

    pub fn is_success(self) -> bool {
        matches!(self, Self::Succeeded)
    }

    fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TrackingLost => "tracking lost",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentRow {
    agent: String,
    description: String,
    status: SubagentRowStatus,
    elapsed: Option<String>,
    elapsed_seconds: Option<u64>,
    tokens: u64,
}

impl SubagentRow {
    pub fn new(agent: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            agent: agent.into(),
            description: description.into(),
            status: SubagentRowStatus::Running,
            elapsed: None,
            elapsed_seconds: None,
            tokens: 0,
        }
    }

    /// Compatibility builder for callers that previously supplied only a
    /// terminal success flag.
    pub fn done(mut self, success: bool) -> Self {
        self.status = if success {
            SubagentRowStatus::Succeeded
        } else {
            SubagentRowStatus::Failed
        };
        self
    }

    pub fn status(mut self, status: SubagentRowStatus) -> Self {
        self.status = status;
        self
    }

    pub fn elapsed(mut self, elapsed: impl Into<String>) -> Self {
        let elapsed = elapsed.into();
        if !elapsed.is_empty() {
            self.elapsed_seconds = parse_elapsed_seconds(&elapsed);
            self.elapsed = Some(elapsed);
        }
        self
    }

    /// Set elapsed time without asking the component to parse host-formatted
    /// text. The tracker remains responsible for a consistent display format.
    pub fn elapsed_seconds(mut self, elapsed_seconds: u64) -> Self {
        self.elapsed_seconds = Some(elapsed_seconds);
        self.elapsed = Some(fmt_elapsed_seconds(elapsed_seconds));
        self
    }

    pub fn tokens(mut self, tokens: u64) -> Self {
        self.tokens = tokens;
        self
    }

    pub fn agent_value(&self) -> &str {
        &self.agent
    }

    pub fn description_value(&self) -> &str {
        &self.description
    }

    pub fn is_done(&self) -> bool {
        self.status.is_terminal()
    }

    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    pub fn status_value(&self) -> SubagentRowStatus {
        self.status
    }

    pub fn elapsed_value(&self) -> Option<&str> {
        self.elapsed.as_deref()
    }

    pub fn elapsed_seconds_value(&self) -> Option<u64> {
        self.elapsed_seconds
    }

    pub fn tokens_value(&self) -> u64 {
        self.tokens
    }

    fn metadata(&self) -> String {
        let elapsed = self.elapsed.as_deref().unwrap_or("0.0s");
        if self.tokens > 0 {
            format!(
                "{} · {elapsed} · ↓ {} tokens",
                self.status.label(),
                fmt_tokens(self.tokens)
            )
        } else {
            format!("{} · {elapsed}", self.status.label())
        }
    }
}

fn row_element<Msg>(
    left: String,
    left_color: Color,
    bold_left: bool,
    right: String,
    right_color: Color,
) -> Element<Msg> {
    let mut left_text = TextElement::new(left).fg(left_color);
    if bold_left {
        left_text = left_text.bold();
    }

    Element::Box(
        BoxElement::new()
            .direction(FlexDirection::Row)
            .child(Element::Text(left_text))
            .child(Element::Text(TextElement::new(" ")))
            .child(Element::Text(TextElement::new(right).fg(right_color))),
    )
}

fn row_color(
    row: &SubagentRow,
    active_color: Color,
    muted_color: Color,
    error_color: Color,
) -> Color {
    match row.status {
        SubagentRowStatus::Running => active_color,
        SubagentRowStatus::Failed => error_color,
        SubagentRowStatus::Succeeded
        | SubagentRowStatus::Cancelled
        | SubagentRowStatus::TrackingLost => muted_color,
    }
}

fn fit_left(left: &str, width: usize, right: &str) -> String {
    let right_width = visible_len(right);
    let max_left = width.saturating_sub(right_width + 3).max(8);
    truncate_visible(left, max_left)
}

fn aggregate_elapsed(rows: &[SubagentRow]) -> String {
    let max_seconds = rows
        .iter()
        .filter_map(|row| {
            row.elapsed_seconds
                .or_else(|| row.elapsed.as_deref().and_then(parse_elapsed_seconds))
        })
        .max()
        .unwrap_or(0);
    fmt_elapsed_seconds(max_seconds)
}

fn parse_elapsed_seconds(text: &str) -> Option<u64> {
    let text = text.trim();
    if let Some((minutes, seconds)) = text.split_once(':') {
        let minutes = minutes.trim().parse::<u64>().ok()?;
        let seconds = seconds.trim().parse::<u64>().ok()?;
        return Some(minutes.saturating_mul(60).saturating_add(seconds));
    }

    let mut total = 0.0f64;
    let mut parts = 0usize;
    for part in text.split_whitespace() {
        total += parse_elapsed_part_seconds(part)?;
        parts += 1;
    }
    (parts > 0).then_some(())?;
    ceil_elapsed_seconds(total)
}

fn parse_elapsed_part_seconds(part: &str) -> Option<f64> {
    if let Some(raw) = part.strip_suffix("ms") {
        return raw.trim().parse::<f64>().ok().map(|value| value / 1000.0);
    }
    if let Some(raw) = part.strip_suffix('s') {
        return raw.trim().parse::<f64>().ok();
    }
    if let Some(raw) = part.strip_suffix('m') {
        return raw.trim().parse::<f64>().ok().map(|value| value * 60.0);
    }
    if let Some(raw) = part.strip_suffix('h') {
        return raw.trim().parse::<f64>().ok().map(|value| value * 3600.0);
    }
    None
}

fn ceil_elapsed_seconds(seconds: f64) -> Option<u64> {
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let seconds = seconds.ceil();
    if seconds >= u64::MAX as f64 {
        Some(u64::MAX)
    } else {
        Some(seconds as u64)
    }
}

fn fmt_elapsed_seconds(seconds: u64) -> String {
    if seconds < 60 {
        format!("{:.1}s", seconds as f64)
    } else if seconds < 3600 {
        format!("{}m {:02}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {:02}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

fn fmt_tokens(tokens: u64) -> String {
    if tokens < 1_000 {
        tokens.to_string()
    } else if tokens < 1_000_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    }
}

fn slugify(text: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for ch in text.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
        if slug.len() >= 24 {
            break;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "parallel-agents".to_string()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn renders_summary_and_running_rows_at_fixed_width() {
        let view = SubagentTracker::new("Extract reusable tui components")
            .slug("extract-tui")
            .row(
                SubagentRow::new("planner", "map panels")
                    .done(true)
                    .elapsed("0.8s")
                    .tokens(900),
            )
            .row(
                SubagentRow::new("coder", "build tracker")
                    .elapsed("1.4s")
                    .tokens(1_500),
            )
            .view(112);
        let plain = strip_ansi(&view);
        let rows = plain.lines().collect::<Vec<_>>();

        assert_eq!(rows.len(), 2);
        assert!(rows[0].contains("extract-tui  Extract reusable tui components"));
        assert!(rows[0].contains("1 running · 1/2 done · 2.0s · ↓ 2.4k tokens"));
        assert!(rows[1].contains("coder  build tracker"));
        assert!(rows[1].contains("1.4s · ↓ 1.5k tokens"));
        for row in rows {
            assert_eq!(visible_len(row), 112);
        }
    }

    #[test]
    fn running_rows_render_explicit_status_and_numeric_elapsed_time() {
        let row = SubagentRow::new("researcher", "collect evidence")
            .elapsed_seconds(65)
            .tokens(720);
        let view = SubagentTracker::new("research").row(row.clone()).view(80);
        let plain = strip_ansi(&view);

        assert_eq!(row.status_value(), SubagentRowStatus::Running);
        assert_eq!(row.elapsed_seconds_value(), Some(65));
        assert_eq!(row.elapsed_value(), Some("1m 05s"));
        assert!(plain.contains("running · 1m 05s · ↓ 720 tokens"), "{plain}");
        assert!(plain.contains("1 running · 0/1 done · 1m 05s"), "{plain}");
    }

    #[test]
    fn legacy_cli_elapsed_formats_remain_parseable() {
        assert_eq!(parse_elapsed_seconds("1m 05s"), Some(65));
        assert_eq!(parse_elapsed_seconds("1h 32m"), Some(5_520));
        assert_eq!(parse_elapsed_seconds("1h 02m 03s"), Some(3_723));

        let rows = vec![
            SubagentRow::new("one", "minute").elapsed("1m 05s"),
            SubagentRow::new("two", "hour").elapsed("1h 32m"),
        ];
        assert_eq!(aggregate_elapsed(&rows), "1h 32m");
    }

    #[test]
    fn overflow_is_reported_as_one_muted_summary_row() {
        let tracker = SubagentTracker::new("many")
            .max_running_rows(2)
            .muted_color(Color::BrightBlack)
            .row(SubagentRow::new("one", "first"))
            .row(SubagentRow::new("two", "second"))
            .row(SubagentRow::new("three", "third"))
            .row(SubagentRow::new("four", "fourth"));
        let view = tracker.view(72);
        let plain = strip_ansi(&view);

        assert_eq!(plain.lines().count(), 4);
        assert!(plain.contains("one  first"), "{plain}");
        assert!(plain.contains("two  second"), "{plain}");
        assert!(!plain.contains("three  third"), "{plain}");
        assert!(plain.contains("… +2 more running"), "{plain}");
        assert!(
            view.contains(&format!(
                "\x1b[{}m… +2 more running",
                Color::BrightBlack.fg_ansi()
            )) || view.contains(&Color::BrightBlack.fg_ansi()),
            "{view:?}"
        );
    }

    #[test]
    fn typed_terminal_statuses_keep_done_builder_compatibility() {
        let succeeded = SubagentRow::new("one", "done").done(true);
        let failed = SubagentRow::new("two", "failed").done(false);
        let cancelled = SubagentRow::new("three", "cancelled").status(SubagentRowStatus::Cancelled);
        let tracking_lost =
            SubagentRow::new("four", "lost").status(SubagentRowStatus::TrackingLost);

        assert!(succeeded.is_done());
        assert!(succeeded.is_success());
        assert_eq!(failed.status_value(), SubagentRowStatus::Failed);
        assert!(failed.is_done());
        assert!(!failed.is_success());
        assert!(cancelled.is_done());
        assert!(tracking_lost.is_done());
    }

    #[test]
    fn empty_tracker_renders_no_rows() {
        let tracker = SubagentTracker::new("No work");

        assert_eq!(tracker.view(80), "");
        assert!(matches!(tracker.element::<()>(), Element::Box(_)));
    }

    #[test]
    fn limits_running_rows() {
        let view = SubagentTracker::new("many")
            .max_running_rows(1)
            .row(SubagentRow::new("one", "first"))
            .row(SubagentRow::new("two", "second"))
            .view(60);
        let plain = strip_ansi(&view);

        assert!(plain.contains("one  first"));
        assert!(!plain.contains("two  second"));
    }

    #[test]
    fn failed_completed_row_uses_error_color_when_rendered_as_child() {
        let tracker = SubagentTracker::new("failures").row(
            SubagentRow::new("reviewer", "check")
                .done(false)
                .elapsed("1s"),
        );
        let element = tracker.child_element::<()>(tracker.rows_value().first().unwrap());

        let Element::Box(row) = element else {
            panic!("expected row box");
        };
        let Element::Text(text) = &row.children[0] else {
            panic!("expected left text");
        };
        assert_eq!(text.style.fg, Some(Color::Red));
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let tracker = SubagentTracker::new("Extract").with_theme(&theme);

        assert_eq!(tracker.accent_color, theme.color(ThemeRole::Primary));
        assert_eq!(tracker.active_color, theme.color(ThemeRole::Secondary));
        assert_eq!(tracker.muted_color, theme.color(ThemeRole::Muted));
        assert_eq!(tracker.error_color, theme.color(ThemeRole::Error));
    }

    #[test]
    fn interactive_tracker_can_hide_redundant_slug() {
        let view = SubagentTracker::new("Audit message stream")
            .slug("audit-message-stream")
            .show_slug(false)
            .row(SubagentRow::new("reviewer", "inspect cards").elapsed_seconds(3))
            .view(72);
        let plain = strip_ansi(&view);

        assert!(plain.contains("Audit message stream"), "{plain}");
        assert!(!plain.contains("audit-message-stream"), "{plain}");
    }

    #[test]
    fn cjk_text_is_truncated_to_width() {
        let view = SubagentTracker::new("提取通用组件并验证终端布局")
            .row(
                SubagentRow::new("coder", "处理中文描述和 very long details")
                    .elapsed("1.2s")
                    .tokens(720),
            )
            .view(36);
        let plain = strip_ansi(&view);

        for row in plain.lines() {
            assert_eq!(visible_len(row), 36, "{row:?}");
        }
    }

    #[test]
    fn summary_token_total_saturates_on_overflow() {
        let tracker = SubagentTracker::new("large token counts")
            .row(SubagentRow::new("one", "done").tokens(u64::MAX))
            .row(SubagentRow::new("two", "done").tokens(1));

        let status = tracker.summary_status();

        assert!(status.contains(&format!("↓ {} tokens", fmt_tokens(u64::MAX))));
    }

    #[test]
    fn non_finite_elapsed_values_are_ignored() {
        assert_eq!(parse_elapsed_seconds("NaNms"), None);
        assert_eq!(parse_elapsed_seconds("infs"), None);
        assert_eq!(parse_elapsed_seconds("-infm"), None);
        assert_eq!(parse_elapsed_seconds("-1s"), None);

        let tracker = SubagentTracker::new("elapsed")
            .row(SubagentRow::new("bad", "ignored").elapsed("infs"))
            .row(SubagentRow::new("good", "used").elapsed("1.2s"));

        assert!(tracker.summary_status().contains("2.0s"));
    }

    #[test]
    fn oversized_indents_are_clamped_to_render_width() {
        let tracker = SubagentTracker::new("Extract")
            .margin(usize::MAX)
            .child_indent(usize::MAX)
            .row(SubagentRow::new("coder", "build").elapsed("1s"));
        let view = tracker.view(8);

        assert_eq!(tracker.margin, MAX_SUBAGENT_MARGIN);
        assert_eq!(tracker.child_indent, MAX_SUBAGENT_CHILD_INDENT);
        assert!(view.lines().all(|line| visible_len(line) == 8));

        let Element::Box(column) = tracker.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Box(summary) = &column.children[0] else {
            panic!("expected summary row");
        };
        let Element::Text(summary_left) = &summary.children[0] else {
            panic!("expected summary text");
        };
        assert_eq!(leading_spaces(&summary_left.content), MAX_SUBAGENT_MARGIN);

        let Element::Box(child) = &column.children[1] else {
            panic!("expected child row");
        };
        let Element::Text(child_left) = &child.children[0] else {
            panic!("expected child text");
        };
        assert_eq!(
            leading_spaces(&child_left.content),
            MAX_SUBAGENT_CHILD_INDENT
        );
    }

    #[test]
    fn oversized_running_row_limit_is_clamped() {
        let tracker = SubagentTracker::new("many")
            .max_running_rows(usize::MAX)
            .row(SubagentRow::new("one", "first"))
            .row(SubagentRow::new("two", "second"));
        let view = tracker.view(40);

        assert_eq!(tracker.max_running_rows, MAX_SUBAGENT_RUNNING_ROWS);
        assert_eq!(tracker.running_rows().len(), 2);
        assert!(view.lines().all(|line| visible_len(line) == 40));
    }

    #[test]
    fn zero_running_row_limit_keeps_one_row_visible() {
        let tracker = SubagentTracker::new("many")
            .max_running_rows(0)
            .row(SubagentRow::new("one", "first"))
            .row(SubagentRow::new("two", "second"));
        let view = tracker.view(40);
        let plain = strip_ansi(&view);

        assert_eq!(tracker.max_running_rows, 1);
        assert_eq!(tracker.running_rows().len(), 1);
        assert!(plain.contains("one  first"));
        assert!(!plain.contains("two  second"));
        assert!(plain.contains("… +1 more running"));

        let Element::Box(column) = tracker.element::<()>() else {
            panic!("expected column");
        };
        assert_eq!(column.children.len(), 3);
    }

    #[test]
    fn element_exposes_structured_rows() {
        let element: Element<()> = SubagentTracker::new("Extract")
            .slug("extract")
            .row(SubagentRow::new("coder", "build").elapsed("1s"))
            .element();

        let Element::Box(column) = element else {
            panic!("expected column");
        };
        assert_eq!(column.children.len(), 2);
    }

    #[test]
    fn slug_defaults_to_parallel_agents_for_non_ascii_titles() {
        let tracker = SubagentTracker::new("提取通用组件");

        assert_eq!(tracker.slug_text(), "parallel-agents");
    }

    fn leading_spaces(value: &str) -> usize {
        value.chars().take_while(|ch| *ch == ' ').count()
    }
}
