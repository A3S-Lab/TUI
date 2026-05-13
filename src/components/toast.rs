use std::time::{Duration, Instant};

use crate::element::{BoxElement, BorderStyle, Element, FlexDirection, TextElement};
use crate::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastKind {
    fn color(&self) -> Color {
        match self {
            ToastKind::Info => Color::Blue,
            ToastKind::Success => Color::Green,
            ToastKind::Warning => Color::Yellow,
            ToastKind::Error => Color::Red,
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            ToastKind::Info => "ℹ",
            ToastKind::Success => "✓",
            ToastKind::Warning => "⚠",
            ToastKind::Error => "✗",
        }
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
        self.max_visible = max;
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
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all toasts.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Render toasts as an Element tree (stacked vertically).
    pub fn element<Msg>(&self) -> Element<Msg> {
        let visible = self.entries.iter().rev().take(self.max_visible);

        let children: Vec<Element<Msg>> = visible
            .map(|entry| {
                let color = entry.kind.color();
                let icon = entry.kind.icon();
                let text = format!(" {} {} ", icon, entry.message);

                Element::Box(
                    BoxElement::new()
                        .direction(FlexDirection::Row)
                        .border(BorderStyle::Rounded)
                        .border_color(color)
                        .child(Element::Text(TextElement::new(text).fg(color))),
                )
            })
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
}
