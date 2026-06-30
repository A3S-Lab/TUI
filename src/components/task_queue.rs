use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, visible_len, Color, Style};

/// Pinned queued-task panel with a running task and ordered queue rows.
///
/// This mirrors the CLI queue strip: a compact completed-count header, an
/// optional running row, and a bounded list of queued messages sorted by
/// priority then sequence.
#[derive(Debug, Clone)]
pub struct TaskQueue {
    completed: usize,
    running: Option<String>,
    queued: Vec<QueuedTask>,
    max_queued_rows: usize,
    margin: usize,
    title: String,
    divider: char,
    running_marker: String,
    queued_marker: String,
    show_when_empty: bool,
    header_color: Color,
    running_color: Color,
    queued_color: Color,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            completed: 0,
            running: None,
            queued: Vec::new(),
            max_queued_rows: 6,
            margin: 2,
            title: "tasks".to_string(),
            divider: '─',
            running_marker: "⏳".to_string(),
            queued_marker: "▱".to_string(),
            show_when_empty: false,
            header_color: Color::BrightBlack,
            running_color: Color::Yellow,
            queued_color: Color::BrightBlack,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        let title = title.into();
        if !title.trim().is_empty() {
            self.title = title;
        }
        self
    }

    pub fn completed(mut self, completed: usize) -> Self {
        self.completed = completed;
        self
    }

    pub fn running(mut self, running: impl Into<String>) -> Self {
        let running = running.into();
        if !running.trim().is_empty() {
            self.running = Some(running);
        }
        self
    }

    pub fn queued(mut self, task: QueuedTask) -> Self {
        self.queued.push(task);
        self
    }

    pub fn queued_tasks(mut self, tasks: Vec<QueuedTask>) -> Self {
        self.queued = tasks;
        self
    }

    pub fn add_queued(&mut self, task: QueuedTask) {
        self.queued.push(task);
    }

    pub fn max_queued_rows(mut self, max_queued_rows: usize) -> Self {
        self.max_queued_rows = max_queued_rows;
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin;
        self
    }

    pub fn divider(mut self, divider: char) -> Self {
        self.divider = divider;
        self
    }

    pub fn running_marker(mut self, marker: impl Into<String>) -> Self {
        let marker = marker.into();
        if !marker.is_empty() {
            self.running_marker = marker;
        }
        self
    }

    pub fn queued_marker(mut self, marker: impl Into<String>) -> Self {
        let marker = marker.into();
        if !marker.is_empty() {
            self.queued_marker = marker;
        }
        self
    }

    pub fn show_when_empty(mut self, show: bool) -> Self {
        self.show_when_empty = show;
        self
    }

    pub fn header_color(mut self, color: Color) -> Self {
        self.header_color = color;
        self
    }

    pub fn running_color(mut self, color: Color) -> Self {
        self.running_color = color;
        self
    }

    pub fn queued_color(mut self, color: Color) -> Self {
        self.queued_color = color;
        self
    }

    pub fn completed_value(&self) -> usize {
        self.completed
    }

    pub fn running_value(&self) -> Option<&str> {
        self.running.as_deref()
    }

    pub fn queued_value(&self) -> &[QueuedTask] {
        &self.queued
    }

    pub fn view(&self, width: u16) -> String {
        let width = width as usize;
        if width == 0 || !self.should_render() {
            return String::new();
        }

        self.render_lines(width).join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        if !self.should_render() {
            return Element::Box(BoxElement::new().direction(FlexDirection::Column));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(self.element_rows()),
        )
    }

    fn should_render(&self) -> bool {
        self.show_when_empty || !self.queued.is_empty()
    }

    fn render_lines(&self, width: usize) -> Vec<String> {
        let mut lines = vec![self.render_header(width)];
        if let Some(running) = self.running.as_deref() {
            lines.push(self.render_task_row(
                &self.running_marker,
                running,
                self.running_color,
                width,
            ));
        }
        lines.extend(self.visible_queued().into_iter().map(|task| {
            self.render_task_row(&self.queued_marker, &task.text, self.queued_color, width)
        }));
        lines
    }

    fn render_header(&self, width: usize) -> String {
        let prefix = format!(
            "{}{} {} · ✓ {} done ",
            " ".repeat(self.margin),
            self.divider,
            self.title.trim(),
            self.completed
        );
        let fill = self
            .divider
            .to_string()
            .repeat(width.saturating_sub(visible_len(&prefix)));
        Style::new()
            .fg(self.header_color)
            .render(&fit_visible(&format!("{prefix}{fill}"), width))
    }

    fn render_task_row(&self, marker: &str, text: &str, color: Color, width: usize) -> String {
        let raw = format!("{}{} {}", " ".repeat(self.margin), marker, text.trim());
        Style::new().fg(color).render(&fit_visible(&raw, width))
    }

    fn visible_queued(&self) -> Vec<&QueuedTask> {
        let mut tasks = self.queued.iter().collect::<Vec<_>>();
        tasks.sort_by_key(|task| (task.priority, task.sequence));
        tasks.truncate(self.max_queued_rows);
        tasks
    }

    fn element_rows<Msg>(&self) -> Vec<Element<Msg>> {
        let mut rows = vec![Element::Text(
            TextElement::new(self.header_text()).fg(self.header_color),
        )];

        if let Some(running) = self.running.as_deref() {
            rows.push(self.task_element(&self.running_marker, running, self.running_color, true));
        }
        rows.extend(self.visible_queued().into_iter().map(|task| {
            self.task_element(&self.queued_marker, &task.text, self.queued_color, false)
        }));
        rows
    }

    fn header_text(&self) -> String {
        format!(
            "{}{} {} · ✓ {} done",
            " ".repeat(self.margin),
            self.divider,
            self.title.trim(),
            self.completed
        )
    }

    fn task_element<Msg>(
        &self,
        marker: &str,
        text: &str,
        color: Color,
        bold: bool,
    ) -> Element<Msg> {
        let mut row = BoxElement::new().direction(FlexDirection::Row);
        row = row.child(Element::Text(TextElement::new(" ".repeat(self.margin))));

        let mut marker_text = TextElement::new(marker).fg(color);
        if bold {
            marker_text = marker_text.bold();
        }
        row = row
            .child(Element::Text(marker_text))
            .child(Element::Text(TextElement::new(" ")))
            .child(Element::Text(TextElement::new(text.trim()).fg(color)));

        Element::Box(row)
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedTask {
    text: String,
    priority: i32,
    sequence: u64,
}

impl QueuedTask {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            priority: 0,
            sequence: 0,
        }
    }

    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn sequence(mut self, sequence: u64) -> Self {
        self.sequence = sequence;
        self
    }

    pub fn text_value(&self) -> &str {
        &self.text
    }

    pub fn priority_value(&self) -> i32 {
        self.priority
    }

    pub fn sequence_value(&self) -> u64 {
        self.sequence
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn renders_running_and_sorted_queued_rows_at_fixed_width() {
        let view = TaskQueue::new()
            .completed(2)
            .running("compile workspace")
            .queued(QueuedTask::new("later").priority(5).sequence(2))
            .queued(QueuedTask::new("first").priority(1).sequence(9))
            .view(48);
        let plain = strip_ansi(&view);
        let rows = plain.lines().collect::<Vec<_>>();

        assert_eq!(rows.len(), 4);
        assert!(rows[0].contains("tasks · ✓ 2 done"));
        assert!(rows[1].contains("⏳ compile workspace"));
        assert!(rows[2].contains("▱ first"));
        assert!(rows[3].contains("▱ later"));
        for row in rows {
            assert_eq!(visible_len(row), 48);
        }
    }

    #[test]
    fn hides_lone_running_task_by_default() {
        let queue = TaskQueue::new().running("compile workspace");

        assert_eq!(queue.view(48), "");
        assert!(strip_ansi(&queue.show_when_empty(true).view(48)).contains("compile workspace"));
    }

    #[test]
    fn limits_queued_rows() {
        let view = TaskQueue::new()
            .max_queued_rows(1)
            .queued(QueuedTask::new("one").sequence(1))
            .queued(QueuedTask::new("two").sequence(2))
            .view(40);
        let plain = strip_ansi(&view);

        assert!(plain.contains("▱ one"));
        assert!(!plain.contains("▱ two"));
    }

    #[test]
    fn cjk_rows_fit_requested_width() {
        let view = TaskQueue::new()
            .completed(7)
            .running("处理中文任务 and long text")
            .queued(QueuedTask::new("继续验证中文宽度"))
            .view(24);
        let plain = strip_ansi(&view);

        for row in plain.lines() {
            assert_eq!(visible_len(row), 24, "{row:?}");
        }
    }

    #[test]
    fn element_produces_structured_rows() {
        let element: Element<()> = TaskQueue::new()
            .completed(1)
            .running("compile")
            .queued(QueuedTask::new("write docs"))
            .element();

        let Element::Box(column) = element else {
            panic!("expected column");
        };
        assert_eq!(column.children.len(), 3);

        let Element::Box(running) = &column.children[1] else {
            panic!("expected running row");
        };
        let Element::Text(marker) = &running.children[1] else {
            panic!("expected marker text");
        };
        assert_eq!(marker.content, "⏳");
        assert_eq!(marker.style.fg, Some(Color::Yellow));
        assert!(marker.style.bold);
    }
}
