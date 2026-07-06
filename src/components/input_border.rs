use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, repeat_visible_char, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_INPUT_BORDER_MARGIN: usize = u16::MAX as usize;
const MAX_INPUT_BORDER_SUFFIX_RULE_WIDTH: usize = u16::MAX as usize;

/// Input-area border line with optional context and mode/effort labels.
///
/// This extracts the CLI input chrome pattern: a margin, a horizontal rule,
/// right-side context text, an accent chip, and an optional animated/rainbow
/// rule variant for temporary mode changes.
#[derive(Debug, Clone)]
pub struct InputBorder {
    margin: usize,
    rule: char,
    context: Option<String>,
    label: Option<String>,
    suffix_rule_width: usize,
    rule_color: Color,
    context_color: Color,
    label_color: Color,
    bold_rule: bool,
    rainbow: Option<RainbowRule>,
}

impl InputBorder {
    pub fn new() -> Self {
        Self {
            margin: 2,
            rule: '─',
            context: None,
            label: None,
            suffix_rule_width: 2,
            rule_color: Color::BrightBlack,
            context_color: Color::BrightBlack,
            label_color: Color::Cyan,
            bold_rule: false,
            rainbow: None,
        }
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin.min(MAX_INPUT_BORDER_MARGIN);
        self
    }

    pub fn rule(mut self, rule: char) -> Self {
        self.rule = rule;
        self
    }

    pub fn context(mut self, context: impl Into<String>) -> Self {
        let context = context.into();
        if !context.is_empty() {
            self.context = Some(context);
        }
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        let label = label.into();
        if !label.is_empty() {
            self.label = Some(label);
        }
        self
    }

    pub fn suffix_rule_width(mut self, width: usize) -> Self {
        self.suffix_rule_width = width.min(MAX_INPUT_BORDER_SUFFIX_RULE_WIDTH);
        self
    }

    pub fn rule_color(mut self, color: Color) -> Self {
        self.rule_color = color;
        self
    }

    pub fn context_color(mut self, color: Color) -> Self {
        self.context_color = color;
        self
    }

    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
        self
    }

    pub fn bold_rule(mut self, enabled: bool) -> Self {
        self.bold_rule = enabled;
        self
    }

    pub fn rainbow(mut self, palette: Vec<Color>, offset: usize) -> Self {
        if !palette.is_empty() {
            self.rainbow = Some(RainbowRule { palette, offset });
        }
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.rule_color = theme.color(ThemeRole::Border);
        self.context_color = theme.color(ThemeRole::Muted);
        self.label_color = theme.color(ThemeRole::Primary);
        self
    }

    pub fn context_value(&self) -> Option<&str> {
        self.context.as_deref()
    }

    pub fn label_value(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn view(&self, width: u16) -> String {
        let width = width as usize;
        if width == 0 {
            return String::new();
        }

        self.segments(width)
            .into_iter()
            .map(|segment| segment.render())
            .collect::<String>()
    }

    pub fn element<Msg>(&self, width: u16) -> Element<Msg> {
        Element::Box(
            BoxElement::new().direction(FlexDirection::Row).children(
                self.segments(width as usize)
                    .into_iter()
                    .map(|segment| Element::Text(segment.text_element()))
                    .collect(),
            ),
        )
    }

    fn segments(&self, width: usize) -> Vec<BorderSegment> {
        if width == 0 {
            return Vec::new();
        }

        let margin = self.margin_for_width(width);
        let inner_width = width.saturating_sub(margin);
        let mut segments = vec![BorderSegment::plain(" ".repeat(margin))];
        if inner_width == 0 {
            return segments;
        }

        if let Some(rainbow) = &self.rainbow {
            segments.extend(self.rainbow_rule_segments(rainbow, inner_width));
            return segments;
        }

        let context = self.context.as_deref().unwrap_or_default();
        let label = self.label.as_deref().unwrap_or_default();
        let suffix_rule_width = self.suffix_rule_width_for_width(inner_width);
        if context.is_empty() && label.is_empty() {
            segments.push(BorderSegment::styled(
                repeat_visible_char(self.rule, inner_width),
                self.rule_color,
                self.bold_rule,
            ));
            return segments;
        }

        let context_width = visible_len(context);
        let label_width = visible_len(label);
        let label_group_width = [
            usize::from(!context.is_empty()),
            context_width,
            usize::from(!label.is_empty()),
            label_width,
            usize::from(!context.is_empty() || !label.is_empty()),
            suffix_rule_width,
        ]
        .into_iter()
        .fold(0usize, usize::saturating_add);
        let prefix_rule_width = inner_width.saturating_sub(label_group_width);

        if prefix_rule_width > 0 {
            segments.push(BorderSegment::styled(
                repeat_visible_char(self.rule, prefix_rule_width),
                self.rule_color,
                self.bold_rule,
            ));
        }

        if !context.is_empty() {
            segments.push(BorderSegment::plain(" "));
            segments.push(BorderSegment::styled(
                context.to_string(),
                self.context_color,
                false,
            ));
        }

        if !label.is_empty() {
            segments.push(BorderSegment::plain(" "));
            segments.push(BorderSegment::styled(
                label.to_string(),
                self.label_color,
                true,
            ));
        }

        if suffix_rule_width > 0 {
            segments.push(BorderSegment::plain(" "));
            segments.push(BorderSegment::styled(
                repeat_visible_char(self.rule, suffix_rule_width),
                self.rule_color,
                self.bold_rule,
            ));
        }

        let rendered = segments
            .iter()
            .map(|segment| segment.render())
            .collect::<String>();
        if visible_len(&rendered) > width {
            vec![BorderSegment::plain(fit_visible(&rendered, width))]
        } else {
            segments
        }
    }

    fn rainbow_rule_segments(&self, rainbow: &RainbowRule, width: usize) -> Vec<BorderSegment> {
        let rule_width = visible_len(&self.rule.to_string());
        if rule_width == 0 {
            return vec![BorderSegment::plain(" ".repeat(width))];
        }

        let rule_count = width / rule_width;
        let remainder = width % rule_width;
        let mut segments = Vec::with_capacity(rule_count + usize::from(remainder > 0));
        for index in 0..rule_count {
            segments.push(BorderSegment::styled(
                self.rule.to_string(),
                rainbow.palette[(index + rainbow.offset) % rainbow.palette.len()],
                true,
            ));
        }
        if remainder > 0 {
            segments.push(BorderSegment::plain(" ".repeat(remainder)));
        }
        segments
    }

    fn margin_for_width(&self, width: usize) -> usize {
        self.margin.min(width).min(MAX_INPUT_BORDER_MARGIN)
    }

    fn suffix_rule_width_for_width(&self, inner_width: usize) -> usize {
        self.suffix_rule_width
            .min(inner_width)
            .min(MAX_INPUT_BORDER_SUFFIX_RULE_WIDTH)
    }
}

impl Default for InputBorder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct RainbowRule {
    palette: Vec<Color>,
    offset: usize,
}

#[derive(Debug, Clone)]
struct BorderSegment {
    text: String,
    color: Option<Color>,
    bold: bool,
}

impl BorderSegment {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
            bold: false,
        }
    }

    fn styled(text: impl Into<String>, color: Color, bold: bool) -> Self {
        Self {
            text: text.into(),
            color: Some(color),
            bold,
        }
    }

    fn render(&self) -> String {
        let Some(color) = self.color else {
            return self.text.clone();
        };
        let mut style = Style::new().fg(color);
        if self.bold {
            style = style.bold();
        }
        style.render(&self.text)
    }

    fn text_element(&self) -> TextElement {
        let mut element = TextElement::new(self.text.clone());
        if let Some(color) = self.color {
            element = element.fg(color);
        }
        if self.bold {
            element = element.bold();
        }
        element
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn renders_context_and_label_at_fixed_width() {
        let rendered = InputBorder::new()
            .context("70% context used")
            .label("◇ high")
            .rule_color(Color::BrightBlack)
            .label_color(Color::Cyan)
            .view(48);
        let plain = strip_ansi(&rendered);

        assert_eq!(visible_len(&rendered), 48);
        assert!(plain.starts_with("  ─"));
        assert!(plain.contains("70% context used"));
        assert!(plain.contains("◇ high"));
        assert!(rendered.contains("\x1b[1;36m◇ high\x1b[0m"));
    }

    #[test]
    fn renders_plain_rule_without_labels() {
        let rendered = InputBorder::new()
            .margin(1)
            .rule('━')
            .rule_color(Color::Green)
            .bold_rule(true)
            .view(12);

        assert_eq!(strip_ansi(&rendered), " ━━━━━━━━━━━");
        assert!(rendered.contains("\x1b[1;32m━━━━━━━━━━━\x1b[0m"));
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let border = InputBorder::new().with_theme(&theme);

        assert_eq!(border.rule_color, theme.color(ThemeRole::Border));
        assert_eq!(border.context_color, theme.color(ThemeRole::Muted));
        assert_eq!(border.label_color, theme.color(ThemeRole::Primary));
    }

    #[test]
    fn custom_wide_rule_glyph_respects_display_width() {
        let plain_rule = InputBorder::new().margin(1).rule('界').view(5);
        let labeled_rule = InputBorder::new()
            .margin(0)
            .rule('界')
            .context("ctx")
            .suffix_rule_width(3)
            .view(10);

        assert_eq!(visible_len(&plain_rule), 5);
        assert_eq!(strip_ansi(&plain_rule), " 界界");
        assert_eq!(visible_len(&labeled_rule), 10);
        assert!(strip_ansi(&labeled_rule).starts_with("界 ctx"));
    }

    #[test]
    fn rainbow_rule_uses_palette_offset_and_ignores_labels() {
        let rendered = InputBorder::new()
            .margin(1)
            .context("hidden")
            .label("also hidden")
            .rainbow(vec![Color::Red, Color::Green], 1)
            .view(5);
        let plain = strip_ansi(&rendered);

        assert_eq!(plain, " ────");
        assert!(rendered.starts_with(" \x1b[1;32m─\x1b[0m\x1b[1;31m─"));
    }

    #[test]
    fn rainbow_wide_rule_glyph_respects_display_width() {
        let rendered = InputBorder::new()
            .margin(1)
            .rule('界')
            .rainbow(vec![Color::Red, Color::Green], 0)
            .view(6);

        assert_eq!(visible_len(&rendered), 6);
        assert_eq!(strip_ansi(&rendered), " 界界 ");
        assert!(rendered.starts_with(" \x1b[1;31m界\x1b[0m\x1b[1;32m界"));
    }

    #[test]
    fn truncates_when_labels_exceed_width() {
        let rendered = InputBorder::new()
            .context("very long context with 中文")
            .label("◇ ultracode")
            .view(20);

        assert_eq!(visible_len(&rendered), 20);
        assert!(strip_ansi(&rendered).contains('…'));
    }

    #[test]
    fn oversized_margin_is_clamped_to_render_width() {
        let border = InputBorder::new().margin(usize::MAX);
        let rendered = border.view(8);

        assert_eq!(border.margin, MAX_INPUT_BORDER_MARGIN);
        assert_eq!(visible_len(&rendered), 8);

        let Element::Box(row) = border.element::<()>(8) else {
            panic!("expected row element");
        };
        let Element::Text(margin) = &row.children[0] else {
            panic!("expected margin text");
        };
        assert_eq!(margin.content.len(), 8);
    }

    #[test]
    fn oversized_suffix_rule_width_is_clamped_to_render_width() {
        let border = InputBorder::new()
            .context("ctx")
            .label("max")
            .suffix_rule_width(usize::MAX);
        let rendered = border.view(12);

        assert_eq!(border.suffix_rule_width, MAX_INPUT_BORDER_SUFFIX_RULE_WIDTH);
        assert_eq!(visible_len(&rendered), 12);
    }

    #[test]
    fn element_produces_structured_segments() {
        let element: Element<()> = InputBorder::new()
            .context("80% context used")
            .label("◇ max")
            .rule_color(Color::BrightBlack)
            .label_color(Color::Cyan)
            .element(40);

        match element {
            Element::Box(row) => {
                assert!(row.children.len() >= 6);
                match &row.children[1] {
                    Element::Text(rule) => {
                        assert!(rule.content.contains('─'));
                        assert_eq!(rule.style.fg, Some(Color::BrightBlack));
                    }
                    _ => panic!("expected rule segment"),
                }
            }
            _ => panic!("expected row element"),
        }
    }
}
