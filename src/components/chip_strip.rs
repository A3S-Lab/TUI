use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, Color, Style};

/// One chip in a [`ChipStrip`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chip {
    label: String,
    color: Option<Color>,
}

impl Chip {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            color: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn label_value(&self) -> &str {
        &self.label
    }

    pub fn color_value(&self) -> Option<Color> {
        self.color
    }
}

/// Compact colored chip strip with an optional active chip.
///
/// This extracts the relay/model picker header pattern from the CLI: each source
/// is rendered in its brand/theme color, while the active source is boxed with a
/// contrasting foreground.
#[derive(Debug, Clone)]
pub struct ChipStrip {
    chips: Vec<Chip>,
    active: Option<usize>,
    margin: usize,
    gap: usize,
    active_fg: Color,
    active_bg: Color,
    inactive_color: Color,
    bold_active: bool,
}

impl ChipStrip {
    pub fn new(chips: Vec<Chip>) -> Self {
        Self {
            chips,
            active: Some(0),
            margin: 2,
            gap: 1,
            active_fg: Color::Black,
            active_bg: Color::Cyan,
            inactive_color: Color::BrightBlack,
            bold_active: true,
        }
    }

    pub fn from_labels(labels: Vec<impl Into<String>>) -> Self {
        Self::new(labels.into_iter().map(Chip::new).collect())
    }

    pub fn chip(mut self, chip: Chip) -> Self {
        self.chips.push(chip);
        self.clamp_active();
        self
    }

    pub fn chips(mut self, chips: Vec<Chip>) -> Self {
        self.chips = chips;
        self.clamp_active();
        self
    }

    pub fn add_chip(&mut self, chip: Chip) {
        self.chips.push(chip);
        self.clamp_active();
    }

    pub fn active(mut self, active: usize) -> Self {
        self.active = if self.chips.is_empty() {
            None
        } else {
            Some(active.min(self.chips.len() - 1))
        };
        self
    }

    pub fn no_active(mut self) -> Self {
        self.active = None;
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin;
        self
    }

    pub fn gap(mut self, gap: usize) -> Self {
        self.gap = gap;
        self
    }

    pub fn active_colors(mut self, fg: Color, bg: Color) -> Self {
        self.active_fg = fg;
        self.active_bg = bg;
        self
    }

    pub fn inactive_color(mut self, color: Color) -> Self {
        self.inactive_color = color;
        self
    }

    pub fn bold_active(mut self, enabled: bool) -> Self {
        self.bold_active = enabled;
        self
    }

    pub fn chips_value(&self) -> &[Chip] {
        &self.chips
    }

    pub fn active_value(&self) -> Option<usize> {
        self.active
    }

    pub fn view(&self, width: u16) -> String {
        let width = width as usize;
        if width == 0 || self.chips.is_empty() {
            return String::new();
        }

        fit_visible(&self.render_raw(), width)
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        if self.chips.is_empty() {
            return Element::Box(BoxElement::new().direction(FlexDirection::Row));
        }

        let mut children = vec![Element::Text(TextElement::new(" ".repeat(self.margin)))];
        for (index, chip) in self.chips.iter().enumerate() {
            if index > 0 && self.gap > 0 {
                children.push(Element::Text(TextElement::new(" ".repeat(self.gap))));
            }
            let mut text = TextElement::new(format!(" {} ", chip.label));
            if self.active == Some(index) {
                text = text.fg(self.active_fg).bg(self.active_bg_for(chip));
                if self.bold_active {
                    text = text.bold();
                }
            } else {
                text = text.fg(self.inactive_fg_for(chip));
            }
            children.push(Element::Text(text));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }

    fn render_raw(&self) -> String {
        let mut out = " ".repeat(self.margin);
        for (index, chip) in self.chips.iter().enumerate() {
            if index > 0 && self.gap > 0 {
                out.push_str(&" ".repeat(self.gap));
            }
            let raw = format!(" {} ", chip.label);
            if self.active == Some(index) {
                let mut style = Style::new().fg(self.active_fg).bg(self.active_bg_for(chip));
                if self.bold_active {
                    style = style.bold();
                }
                out.push_str(&style.render(&raw));
            } else {
                out.push_str(&Style::new().fg(self.inactive_fg_for(chip)).render(&raw));
            }
        }
        out
    }

    fn active_bg_for(&self, chip: &Chip) -> Color {
        chip.color.unwrap_or(self.active_bg)
    }

    fn inactive_fg_for(&self, chip: &Chip) -> Color {
        chip.color.unwrap_or(self.inactive_color)
    }

    fn clamp_active(&mut self) {
        self.active = self.active.and_then(|active| {
            (!self.chips.is_empty()).then_some(active.min(self.chips.len() - 1))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn renders_active_chip_with_chip_background() {
        let rendered = ChipStrip::new(vec![
            Chip::new("a3s-code").color(Color::Cyan),
            Chip::new("Codex").color(Color::Rgb(115, 218, 202)),
        ])
        .active(1)
        .view(48);
        let plain = strip_ansi(&rendered);

        assert!(plain.starts_with("   a3s-code   Codex "), "{plain:?}");
        assert_eq!(visible_len(&rendered), 48);
        assert!(rendered.contains("1;30;48;2;115;218;202"));
    }

    #[test]
    fn inactive_chips_use_their_colors() {
        let rendered = ChipStrip::new(vec![
            Chip::new("Claude").color(Color::Yellow),
            Chip::new("Codex").color(Color::Cyan),
        ])
        .active(0)
        .view(40);

        assert!(rendered.contains("\x1b[36m Codex \x1b[0m"));
    }

    #[test]
    fn active_index_is_clamped() {
        let strip = ChipStrip::from_labels(vec!["one", "two"]).active(99);

        assert_eq!(strip.active_value(), Some(1));
    }

    #[test]
    fn empty_strip_renders_no_text() {
        let strip = ChipStrip::new(Vec::new());

        assert_eq!(strip.view(80), "");
        assert!(matches!(strip.element::<()>(), Element::Box(_)));
    }

    #[test]
    fn cjk_labels_fit_requested_width() {
        let rendered = ChipStrip::new(vec![
            Chip::new("模型").color(Color::Cyan),
            Chip::new("会话").color(Color::Yellow),
            Chip::new("网关").color(Color::Magenta),
        ])
        .active(2)
        .view(24);

        assert_eq!(visible_len(&rendered), 24);
        assert!(strip_ansi(&rendered).contains("网关"));
    }

    #[test]
    fn element_produces_structured_chip_styles() {
        let element: Element<()> = ChipStrip::new(vec![
            Chip::new("Claude").color(Color::Yellow),
            Chip::new("Codex").color(Color::Cyan),
        ])
        .active(0)
        .element();

        let Element::Box(row) = element else {
            panic!("expected row");
        };
        let Element::Text(active) = &row.children[1] else {
            panic!("expected active chip");
        };
        assert_eq!(active.content, " Claude ");
        assert_eq!(active.style.fg, Some(Color::Black));
        assert_eq!(active.style.bg, Some(Color::Yellow));
        assert!(active.style.bold);
    }
}
