use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_MODE_LINE_MARGIN: usize = u16::MAX as usize;

/// Current run/input mode row with muted shortcut hints.
///
/// This covers CLI footers such as `⏵⏵ auto mode on (...)`: a styled mode
/// segment followed by lower-emphasis hints, padded or truncated to a stable
/// terminal width.
#[derive(Debug, Clone)]
pub struct ModeLine {
    mode: String,
    glyph: String,
    suffix: String,
    hints: String,
    margin: usize,
    mode_color: Color,
    hint_color: Color,
}

impl ModeLine {
    pub fn new(mode: impl Into<String>) -> Self {
        Self {
            mode: mode.into(),
            glyph: String::new(),
            suffix: "mode on".to_string(),
            hints: String::new(),
            margin: 2,
            mode_color: Color::Cyan,
            hint_color: Color::BrightBlack,
        }
    }

    pub fn glyph(mut self, glyph: impl Into<String>) -> Self {
        self.glyph = glyph.into();
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    pub fn hints(mut self, hints: impl Into<String>) -> Self {
        self.hints = hints.into();
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin.min(MAX_MODE_LINE_MARGIN);
        self
    }

    pub fn mode_color(mut self, color: Color) -> Self {
        self.mode_color = color;
        self
    }

    pub fn hint_color(mut self, color: Color) -> Self {
        self.hint_color = color;
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.mode_color = theme.color(ThemeRole::Primary);
        self.hint_color = theme.color(ThemeRole::Muted);
        self
    }

    pub fn mode_value(&self) -> &str {
        &self.mode
    }

    pub fn glyph_value(&self) -> &str {
        &self.glyph
    }

    pub fn hints_value(&self) -> &str {
        &self.hints
    }

    pub fn view(&self, width: u16) -> String {
        let width = width as usize;
        if width == 0 {
            return String::new();
        }
        fit_visible(&self.render_raw(self.margin_for_width(width)), width)
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut row = BoxElement::new()
            .direction(FlexDirection::Row)
            .child(Element::Text(TextElement::new(
                " ".repeat(self.margin_for_element()),
            )))
            .child(Element::Text(
                TextElement::new(self.mode_segment())
                    .fg(self.mode_color)
                    .bold(),
            ));

        if !self.hints.is_empty() {
            row = row.child(Element::Text(
                TextElement::new(format!(" {}", self.hints)).fg(self.hint_color),
            ));
        }

        Element::Box(row)
    }

    fn render_raw(&self, margin: usize) -> String {
        let mut raw = format!(
            "{}{}",
            " ".repeat(margin),
            Style::new()
                .fg(self.mode_color)
                .bold()
                .render(&self.mode_segment())
        );
        if !self.hints.is_empty() {
            raw.push(' ');
            raw.push_str(&Style::new().fg(self.hint_color).render(&self.hints));
        }
        raw
    }

    fn margin_for_width(&self, width: usize) -> usize {
        self.margin.min(width).min(MAX_MODE_LINE_MARGIN)
    }

    fn margin_for_element(&self) -> usize {
        self.margin.min(MAX_MODE_LINE_MARGIN)
    }

    fn mode_segment(&self) -> String {
        let mut parts = Vec::new();
        if !self.glyph.is_empty() {
            parts.push(self.glyph.as_str());
        }
        if !self.mode.is_empty() {
            parts.push(self.mode.as_str());
        }
        if !self.suffix.is_empty() {
            parts.push(self.suffix.as_str());
        }
        parts.join(" ")
    }
}

impl Default for ModeLine {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn renders_mode_and_hints_at_fixed_width() {
        let rendered = ModeLine::new("auto")
            .glyph("⏵⏵")
            .hints("(shift+tab to cycle) · /help · esc")
            .mode_color(Color::Green)
            .view(64);
        let plain = strip_ansi(&rendered);

        assert_eq!(visible_len(&rendered), 64);
        assert!(plain.starts_with("  ⏵⏵ auto mode on"));
        assert!(plain.contains("/help"));
        assert!(rendered.contains("\x1b[1;32m⏵⏵ auto mode on\x1b[0m"));
    }

    #[test]
    fn truncates_long_hints_by_display_width() {
        let rendered = ModeLine::new("default")
            .glyph("❯")
            .hints("(very long hint row with 中文 payload)")
            .view(28);

        assert_eq!(visible_len(&rendered), 28);
        assert!(strip_ansi(&rendered).contains('…'));
    }

    #[test]
    fn can_render_without_suffix_or_hints() {
        let rendered = ModeLine::new("insert")
            .glyph("--")
            .suffix("")
            .margin(1)
            .view(20);

        assert_eq!(strip_ansi(&rendered).trim_end(), " -- insert");
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let line = ModeLine::new("auto").with_theme(&theme);

        assert_eq!(line.mode_color, theme.color(ThemeRole::Primary));
        assert_eq!(line.hint_color, theme.color(ThemeRole::Muted));
    }

    #[test]
    fn oversized_margin_is_clamped_to_render_width() {
        let line = ModeLine::new("auto").margin(usize::MAX);
        let rendered = line.view(8);

        assert_eq!(line.margin, MAX_MODE_LINE_MARGIN);
        assert_eq!(visible_len(&rendered), 8);

        let Element::Box(row) = line.element::<()>() else {
            panic!("expected row element");
        };
        let Element::Text(margin) = &row.children[0] else {
            panic!("expected margin text");
        };
        assert_eq!(margin.content.len(), MAX_MODE_LINE_MARGIN);
    }

    #[test]
    fn element_produces_styled_mode_and_hint_segments() {
        let element: Element<()> = ModeLine::new("plan")
            .glyph("⏵")
            .hints("/help")
            .mode_color(Color::Cyan)
            .hint_color(Color::BrightBlack)
            .element();

        match element {
            Element::Box(row) => {
                assert_eq!(row.children.len(), 3);
                match &row.children[1] {
                    Element::Text(mode) => {
                        assert_eq!(mode.content, "⏵ plan mode on");
                        assert_eq!(mode.style.fg, Some(Color::Cyan));
                        assert!(mode.style.bold);
                    }
                    _ => panic!("expected mode segment"),
                }
                match &row.children[2] {
                    Element::Text(hints) => {
                        assert_eq!(hints.content, " /help");
                        assert_eq!(hints.style.fg, Some(Color::BrightBlack));
                    }
                    _ => panic!("expected hints segment"),
                }
            }
            _ => panic!("expected row element"),
        }
    }
}
