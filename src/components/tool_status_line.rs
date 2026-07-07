use crate::style::{truncate_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_TOOL_STATUS_LINE_MARGIN: usize = u16::MAX as usize;

/// Single-line tool status with a colored marker, action label, detail, and suffix.
///
/// This covers transcript rows such as `• Running cargo test...` and
/// `• Ran src/main.rs`, including ANSI-styled details like shell commands.
#[derive(Debug, Clone)]
pub struct ToolStatusLine {
    label: String,
    detail: Option<ToolStatusDetail>,
    marker: String,
    suffix: String,
    margin: usize,
    marker_color: Color,
    label_color: Option<Color>,
    detail_color: Color,
    suffix_color: Color,
    marker_bold: bool,
    label_bold: bool,
}

impl ToolStatusLine {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: None,
            marker: "•".to_string(),
            suffix: String::new(),
            margin: 2,
            marker_color: Color::Green,
            label_color: None,
            detail_color: Color::BrightBlack,
            suffix_color: Color::BrightBlack,
            marker_bold: true,
            label_bold: true,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(ToolStatusDetail::Plain(detail.into()));
        self
    }

    pub fn styled_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(ToolStatusDetail::Styled(detail.into()));
        self
    }

    pub fn marker(mut self, marker: impl Into<String>) -> Self {
        let marker = marker.into();
        if !marker.is_empty() {
            self.marker = marker;
        }
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin.min(MAX_TOOL_STATUS_LINE_MARGIN);
        self
    }

    pub fn marker_color(mut self, color: Color) -> Self {
        self.marker_color = color;
        self
    }

    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = Some(color);
        self
    }

    pub fn detail_color(mut self, color: Color) -> Self {
        self.detail_color = color;
        self
    }

    pub fn suffix_color(mut self, color: Color) -> Self {
        self.suffix_color = color;
        self
    }

    pub fn marker_bold(mut self, bold: bool) -> Self {
        self.marker_bold = bold;
        self
    }

    pub fn label_bold(mut self, bold: bool) -> Self {
        self.label_bold = bold;
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.marker_color = theme.color(ThemeRole::Primary);
        self.label_color = Some(theme.color(ThemeRole::Foreground));
        self.detail_color = theme.color(ThemeRole::Muted);
        self.suffix_color = theme.color(ThemeRole::Muted);
        self
    }

    pub fn label_value(&self) -> &str {
        &self.label
    }

    pub fn view(&self, width: u16) -> String {
        let width = width as usize;
        if width == 0 {
            return String::new();
        }

        let margin = " ".repeat(self.margin.min(width));
        let marker = styled(&self.marker, Some(self.marker_color), self.marker_bold);
        let label = styled(&self.label, self.label_color, self.label_bold);
        let base = format!("{margin}{marker} {label}");
        let suffix = self.render_suffix();

        let Some(detail) = self.detail.as_ref().filter(|detail| !detail.is_empty()) else {
            return truncate_visible(&format!("{base}{suffix}"), width);
        };

        let detail_width = width
            .saturating_sub(visible_len(&base) + 1 + visible_len(&suffix))
            .max(1);
        let detail = match detail {
            ToolStatusDetail::Plain(value) => Style::new()
                .fg(self.detail_color)
                .render(&truncate_visible(value, detail_width)),
            ToolStatusDetail::Styled(value) => truncate_visible(value, detail_width),
        };

        truncate_visible(&format!("{base} {detail}{suffix}"), width)
    }

    fn render_suffix(&self) -> String {
        if self.suffix.is_empty() {
            String::new()
        } else {
            Style::new().fg(self.suffix_color).render(&self.suffix)
        }
    }
}

impl Default for ToolStatusLine {
    fn default() -> Self {
        Self::new("")
    }
}

#[derive(Debug, Clone)]
enum ToolStatusDetail {
    Plain(String),
    Styled(String),
}

impl ToolStatusDetail {
    fn is_empty(&self) -> bool {
        match self {
            Self::Plain(value) | Self::Styled(value) => value.is_empty(),
        }
    }
}

fn styled(text: &str, color: Option<Color>, bold: bool) -> String {
    let mut style = Style::new();
    if let Some(color) = color {
        style = style.fg(color);
    }
    if bold {
        style = style.bold();
    }
    style.render(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn renders_marker_label_and_plain_detail() {
        let rendered = ToolStatusLine::new("Ran")
            .detail("npm test")
            .marker_color(Color::Cyan)
            .detail_color(Color::BrightBlack)
            .view(32);
        let plain = strip_ansi(&rendered);

        assert_eq!(plain, "  • Ran npm test");
        assert!(rendered.contains("\x1b[1;36m•\x1b[0m"));
    }

    #[test]
    fn preserves_styled_detail_and_suffix() {
        let detail = format!(
            "{} {}",
            Style::new().fg(Color::Cyan).bold().render("cargo"),
            Style::new().fg(Color::Yellow).render("test")
        );
        let rendered = ToolStatusLine::new("Running")
            .styled_detail(detail)
            .suffix("…")
            .suffix_color(Color::BrightBlack)
            .label_bold(false)
            .view(40);
        let plain = strip_ansi(&rendered);

        assert_eq!(plain, "  • Running cargo test…");
        assert!(rendered.contains("\x1b[1;36mcargo\x1b[0m"));
        assert!(rendered.contains("\x1b[33mtest\x1b[0m"));
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let line = ToolStatusLine::new("Running").with_theme(&theme);

        assert_eq!(line.marker_color, theme.color(ThemeRole::Primary));
        assert_eq!(line.label_color, Some(theme.color(ThemeRole::Foreground)));
        assert_eq!(line.detail_color, theme.color(ThemeRole::Muted));
        assert_eq!(line.suffix_color, theme.color(ThemeRole::Muted));
    }

    #[test]
    fn truncates_detail_before_suffix_to_width() {
        let rendered = ToolStatusLine::new("Running")
            .detail("a very long command with arguments")
            .suffix("…")
            .view(24);
        let plain = strip_ansi(&rendered);

        assert_eq!(visible_len(&rendered), 24);
        assert!(plain.starts_with("  • Running "));
        assert!(plain.ends_with('…'));
    }
}
