use std::time::{Duration, Instant};

use crate::element::{BorderStyle, BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, Color, Style};

const MAX_TOAST_VISIBLE: usize = u16::MAX as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastKind {
    pub fn color(self) -> Color {
        match self {
            ToastKind::Info => Color::Blue,
            ToastKind::Success => Color::Green,
            ToastKind::Warning => Color::Yellow,
            ToastKind::Error => Color::Red,
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            ToastKind::Info => "ℹ",
            ToastKind::Success => "✓",
            ToastKind::Warning => "⚠",
            ToastKind::Error => "✗",
        }
    }
}

/// Single toast notification with line and Element rendering.
#[derive(Debug, Clone)]
pub struct Toast {
    kind: ToastKind,
    message: String,
    color: Option<Color>,
}

impl Toast {
    pub fn new(kind: ToastKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            color: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn view(&self) -> String {
        Style::new().fg(self.resolved_color()).render(&format!(
            "{} {}",
            self.kind.icon(),
            self.message.trim()
        ))
    }

    pub fn view_with_width(&self, width: u16) -> String {
        fit_visible(&self.view(), width as usize)
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let text = format!(" {} {} ", self.kind.icon(), self.message.trim());
        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .border(BorderStyle::Rounded)
                .border_color(self.resolved_color())
                .child(Element::Text(
                    TextElement::new(text).fg(self.resolved_color()),
                )),
        )
    }

    fn resolved_color(&self) -> Color {
        self.color.unwrap_or_else(|| self.kind.color())
    }
}

struct ToastEntry {
    kind: ToastKind,
    message: String,
    created_at: Instant,
    duration: Duration,
}

/// A toast notification manager that displays timed messages.
///
/// ```rust
/// use a3s_tui::components::toast::{ToastManager, ToastKind};
/// use std::time::Duration;
///
/// let mut toasts = ToastManager::new();
/// toasts.push(ToastKind::Success, "File saved!");
/// toasts.push_with_duration(ToastKind::Error, "Connection failed", Duration::from_secs(5));
/// // Call toasts.tick() each frame to expire old toasts
/// ```
pub struct ToastManager {
    entries: Vec<ToastEntry>,
    default_duration: Duration,
    max_visible: usize,
}

impl ToastManager {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            default_duration: Duration::from_secs(3),
            max_visible: 5,
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.default_duration = duration;
        self
    }

    pub fn with_max_visible(mut self, max: usize) -> Self {
        self.max_visible = max.min(MAX_TOAST_VISIBLE);
        self
    }

    /// Add a toast with the default duration.
    pub fn push(&mut self, kind: ToastKind, message: impl Into<String>) {
        self.entries.push(ToastEntry {
            kind,
            message: message.into(),
            created_at: Instant::now(),
            duration: self.default_duration,
        });
    }

    /// Add a toast with a custom duration.
    pub fn push_with_duration(
        &mut self,
        kind: ToastKind,
        message: impl Into<String>,
        duration: Duration,
    ) {
        self.entries.push(ToastEntry {
            kind,
            message: message.into(),
            created_at: Instant::now(),
            duration,
        });
    }

    /// Remove expired toasts. Call once per frame.
    pub fn tick(&mut self) {
        self.entries.retain(|e| e.created_at.elapsed() < e.duration);
    }

    /// Number of currently visible toasts.
    pub fn len(&self) -> usize {
        self.entries.len().min(self.max_visible)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all toasts.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Render toasts as an Element tree (stacked vertically).
    pub fn element<Msg>(&self) -> Element<Msg> {
        let visible = self.entries.iter().rev().take(self.max_visible);

        let children: Vec<Element<Msg>> = visible
            .map(|entry| Toast::new(entry.kind, &entry.message).element())
            .collect();

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .gap(1)
                .children(children),
        )
    }
}

impl Default for ToastManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn toast_view_renders_icon_and_message() {
        let rendered = Toast::new(ToastKind::Success, "saved").view();

        assert_eq!(strip_ansi(&rendered), "✓ saved");
        assert!(rendered.contains("\x1b[32m"));
    }

    #[test]
    fn toast_view_uses_custom_color() {
        let rendered = Toast::new(ToastKind::Warning, "careful")
            .color(Color::Rgb(245, 166, 35))
            .view();

        assert_eq!(strip_ansi(&rendered), "⚠ careful");
        assert!(rendered.contains("\x1b[38;2;245;166;35m"));
    }

    #[test]
    fn toast_view_with_width_bounds_visible_length() {
        let rendered = Toast::new(ToastKind::Error, "a very long message").view_with_width(8);

        assert_eq!(visible_len(&rendered), 8);
    }

    #[test]
    fn toast_element_contains_icon_and_message() {
        let Element::Box(row) = Toast::new(ToastKind::Info, "hello").element::<()>() else {
            panic!("expected row");
        };

        assert_eq!(row.children.len(), 1);
        assert_eq!(row.children[0].text_content(), Some(" ℹ hello "));
    }

    #[test]
    fn push_and_len() {
        let mut tm = ToastManager::new();
        assert!(tm.is_empty());
        tm.push(ToastKind::Info, "hello");
        assert_eq!(tm.len(), 1);
        tm.push(ToastKind::Error, "oops");
        assert_eq!(tm.len(), 2);
    }

    #[test]
    fn tick_expires_old_toasts() {
        let mut tm = ToastManager::new().with_duration(Duration::from_millis(1));
        tm.push(ToastKind::Success, "done");
        std::thread::sleep(Duration::from_millis(5));
        tm.tick();
        assert!(tm.is_empty());
    }

    #[test]
    fn clear_removes_all() {
        let mut tm = ToastManager::new();
        tm.push(ToastKind::Info, "a");
        tm.push(ToastKind::Info, "b");
        tm.clear();
        assert!(tm.is_empty());
    }

    #[test]
    fn custom_duration_per_toast() {
        let mut tm = ToastManager::new().with_duration(Duration::from_secs(10));
        tm.push_with_duration(ToastKind::Warning, "quick", Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        tm.tick();
        assert!(tm.is_empty());
    }

    #[test]
    fn toast_kind_colors() {
        assert_eq!(ToastKind::Info.color(), Color::Blue);
        assert_eq!(ToastKind::Success.color(), Color::Green);
        assert_eq!(ToastKind::Warning.color(), Color::Yellow);
        assert_eq!(ToastKind::Error.color(), Color::Red);
    }

    #[test]
    fn oversized_visible_limit_is_clamped() {
        let mut tm = ToastManager::new().with_max_visible(usize::MAX);
        tm.push(ToastKind::Info, "one");
        tm.push(ToastKind::Info, "two");

        assert_eq!(tm.max_visible, MAX_TOAST_VISIBLE);

        let Element::Box(column) = tm.element::<()>() else {
            panic!("expected column");
        };
        assert_eq!(column.children.len(), 2);
    }

    #[test]
    fn len_counts_visible_toasts() {
        let mut tm = ToastManager::new().with_max_visible(1);
        tm.push(ToastKind::Info, "one");
        tm.push(ToastKind::Info, "two");

        assert_eq!(tm.len(), 1);

        let Element::Box(column) = tm.element::<()>() else {
            panic!("expected column");
        };
        assert_eq!(column.children.len(), 1);
    }

    #[test]
    fn zero_visible_toasts_is_empty_for_rendering() {
        let mut tm = ToastManager::new().with_max_visible(0);
        tm.push(ToastKind::Info, "hidden");

        assert_eq!(tm.len(), 0);
        assert!(tm.is_empty());

        let Element::Box(column) = tm.element::<()>() else {
            panic!("expected column");
        };
        assert!(column.children.is_empty());
    }
}
