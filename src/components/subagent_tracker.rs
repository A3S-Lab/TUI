use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};

/// Durable status rows for parallel subagents or background workers.
///
/// This extracts the common CLI pattern of a summary row with aggregate
/// progress plus live child rows for workers that are still running.
#[derive(Debug, Clone)]
pub struct SubagentTracker {
    title: String,
    slug: Option<String>,
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
        self.max_running_rows = max_running_rows;
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin;
        self
    }

    pub fn child_indent(mut self, child_indent: usize) -> Self {
        self.child_indent = child_indent;
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
            row_color(row, self.active_color, self.error_color),
            false,
            right,
            width,
        )
    }

    fn render_aligned(
        &self,
        left: String,
        left_color: Color,
        bold_left: bool,
        right: String,
        width: usize,
    ) -> String {
        let pad = width.saturating_sub(visible_len(&left) + visible_len(&right) + 1);
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
        let raw = format!(
            "{}{} {}  {}",
            " ".repeat(self.margin),
            self.marker,
            self.slug_text(),
            self.title.trim()
        );
        fit_left(&raw, width, right)
    }

    fn child_left(&self, row: &SubagentRow, width: usize, right: &str) -> String {
        let raw = format!(
            "{}{} {}  {}",
            " ".repeat(self.child_indent),
            self.marker,
            row.agent,
            row.description
        );
        fit_left(&raw, width, right)
    }

    fn summary_status(&self) -> String {
        let total = self.rows.len();
        let done = self.rows.iter().filter(|row| row.done).count();
        let running = total.saturating_sub(done);
        let tokens = self.rows.iter().map(|row| row.tokens).sum::<u64>();
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
            .filter(|row| !row.done)
            .take(self.max_running_rows)
            .collect()
    }

    fn element_rows<Msg>(&self) -> Vec<Element<Msg>> {
        let mut rows = vec![self.summary_element()];
        rows.extend(
            self.running_rows()
                .into_iter()
                .map(|row| self.child_element(row)),
        );
        rows
    }

    fn summary_element<Msg>(&self) -> Element<Msg> {
        let right = self.summary_status();
        let left = format!(
            "{}{} {}  {}",
            " ".repeat(self.margin),
            self.marker,
            self.slug_text(),
            self.title.trim()
        );
        row_element(left, self.accent_color, true, right, self.muted_color)
    }

    fn child_element<Msg>(&self, row: &SubagentRow) -> Element<Msg> {
        let left = format!(
            "{}{} {}  {}",
            " ".repeat(self.child_indent),
            self.marker,
            row.agent,
            row.description
        );
        row_element(
            left,
            row_color(row, self.active_color, self.error_color),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentRow {
    agent: String,
    description: String,
    done: bool,
    success: bool,
    elapsed: Option<String>,
    tokens: u64,
}

impl SubagentRow {
    pub fn new(agent: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            agent: agent.into(),
            description: description.into(),
            done: false,
            success: true,
            elapsed: None,
            tokens: 0,
        }
    }

    pub fn done(mut self, success: bool) -> Self {
        self.done = true;
        self.success = success;
        self
    }

    pub fn elapsed(mut self, elapsed: impl Into<String>) -> Self {
        let elapsed = elapsed.into();
        if !elapsed.is_empty() {
            self.elapsed = Some(elapsed);
        }
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
        self.done
    }

    pub fn is_success(&self) -> bool {
        self.success
    }

    pub fn elapsed_value(&self) -> Option<&str> {
        self.elapsed.as_deref()
    }

    pub fn tokens_value(&self) -> u64 {
        self.tokens
    }

    fn metadata(&self) -> String {
        let elapsed = self.elapsed.as_deref().unwrap_or("0.0s");
        if self.tokens > 0 {
            format!("{elapsed} · ↓ {} tokens", fmt_tokens(self.tokens))
        } else {
            elapsed.to_string()
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

fn row_color(row: &SubagentRow, active_color: Color, error_color: Color) -> Color {
    if row.done && !row.success {
        error_color
    } else {
        active_color
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
        .filter_map(|row| row.elapsed.as_deref())
        .filter_map(parse_elapsed_seconds)
        .max()
        .unwrap_or(0);
    fmt_elapsed_seconds(max_seconds)
}

fn parse_elapsed_seconds(text: &str) -> Option<u64> {
    let text = text.trim();
    if let Some(raw) = text.strip_suffix("ms") {
        let millis = raw.trim().parse::<f64>().ok()?;
        return Some((millis / 1000.0).ceil() as u64);
    }
    if let Some(raw) = text.strip_suffix('s') {
        let seconds = raw.trim().parse::<f64>().ok()?;
        return Some(seconds.ceil() as u64);
    }
    if let Some(raw) = text.strip_suffix('m') {
        let minutes = raw.trim().parse::<f64>().ok()?;
        return Some((minutes * 60.0).ceil() as u64);
    }
    if let Some((minutes, seconds)) = text.split_once(':') {
        let minutes = minutes.trim().parse::<u64>().ok()?;
        let seconds = seconds.trim().parse::<u64>().ok()?;
        return Some(minutes.saturating_mul(60).saturating_add(seconds));
    }
    None
}

fn fmt_elapsed_seconds(seconds: u64) -> String {
    if seconds < 60 {
        format!("{:.1}s", seconds as f64)
    } else {
        format!("{}m{}s", seconds / 60, seconds % 60)
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
}
