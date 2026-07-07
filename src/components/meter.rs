use crate::style::{repeat_visible_char, truncate_visible, visible_len, Color, Style};

const MAX_METER_WIDTH: usize = u16::MAX as usize;

#[derive(Debug, Clone)]
pub struct Meter {
    label: String,
    value: f64,
    max: f64,
    width: usize,
    fg: Color,
    empty_fg: Color,
    fill: char,
    empty: char,
    show_value: bool,
}

impl Meter {
    pub fn new(value: f64) -> Self {
        Self {
            label: String::new(),
            value,
            max: 100.0,
            width: 32,
            fg: Color::Cyan,
            empty_fg: Color::BrightBlack,
            fill: '█',
            empty: '░',
            show_value: true,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = Self::normalize_max(max);
        self
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = width.clamp(1, MAX_METER_WIDTH);
        self
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    pub fn empty_fg(mut self, color: Color) -> Self {
        self.empty_fg = color;
        self
    }

    pub fn glyphs(mut self, fill: char, empty: char) -> Self {
        self.fill = fill;
        self.empty = empty;
        self
    }

    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    pub fn view(&self) -> String {
        let value = if self.value.is_finite() {
            self.value
        } else {
            0.0
        };
        let max = Self::normalize_max(self.max);
        let ratio = (value / max).clamp(0.0, 1.0);
        let value_label = if !self.show_value {
            String::new()
        } else if (max - 100.0).abs() < f64::EPSILON {
            format!("{value:>5.1}%")
        } else {
            format!("{value:>6.1}")
        };
        let prefix = match (self.label.is_empty(), self.show_value) {
            (true, true) => format!("{value_label} "),
            (true, false) => String::new(),
            (false, true) => format!("{} {value_label} ", self.label),
            (false, false) => format!("{} ", self.label),
        };

        let prefix_width = visible_len(&prefix).min(self.width);
        let bar_width = self.width.saturating_sub(prefix_width);
        if bar_width == 0 {
            return truncate_visible(&prefix, self.width);
        }

        let filled = ((bar_width as f64) * ratio).round() as usize;
        let filled = filled.min(bar_width);
        let empty = bar_width.saturating_sub(filled);
        let bar = format!(
            "{}{}",
            Style::new()
                .fg(self.fg)
                .render(&repeat_visible_char(self.fill, filled)),
            Style::new()
                .fg(self.empty_fg)
                .render(&repeat_visible_char(self.empty, empty))
        );

        format!("{prefix}{bar}")
    }

    pub fn plain(&self) -> String {
        crate::style::strip_ansi(&self.view())
    }

    fn normalize_max(max: f64) -> f64 {
        if max.is_finite() {
            max.max(f64::EPSILON)
        } else {
            f64::EPSILON
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn renders_fixed_width() {
        let line = Meter::new(25.0).label("CPU").width(20).view();

        assert_eq!(visible_len(&line), 20);
        assert!(strip_ansi(&line).contains("25.0%"));
    }

    #[test]
    fn clamps_to_full_bar() {
        let line = Meter::new(200.0).label("MEM").width(16).plain();

        assert_eq!(visible_len(&line), 16);
        assert!(!line.contains('░'));
    }

    #[test]
    fn truncates_when_prefix_exceeds_width() {
        let line = Meter::new(1.0).label("very-long-label").width(8).plain();

        assert_eq!(visible_len(&line), 8);
    }

    #[test]
    fn can_render_bar_without_value_label() {
        let line = Meter::new(50.0)
            .width(6)
            .glyphs('▰', '▱')
            .show_value(false)
            .plain();

        assert_eq!(visible_len(&line), 6);
        assert_eq!(line, "▰▰▰▱▱▱");
    }

    #[test]
    fn non_finite_max_is_treated_as_minimum_positive_range() {
        let line = Meter::new(1.0).max(f64::NAN).width(12).plain();

        assert_eq!(visible_len(&line), 12);
        assert!(!line.contains('░'));
    }

    #[test]
    fn custom_wide_glyphs_fill_bar_by_display_width() {
        let line = Meter::new(50.0)
            .label("CPU")
            .width(16)
            .glyphs('界', '·')
            .view();
        let plain = strip_ansi(&line);

        assert_eq!(visible_len(&line), 16);
        assert_eq!(visible_len(&plain), 16);
        assert!(plain.contains("界 "));
        assert!(plain.ends_with("··"));
    }

    #[test]
    fn custom_zero_width_glyphs_fall_back_to_spaces() {
        let line = Meter::new(50.0)
            .width(12)
            .glyphs('\u{301}', '\u{301}')
            .plain();

        assert_eq!(visible_len(&line), 12);
        assert!(line.ends_with("     "));
    }

    #[test]
    fn oversized_width_is_clamped() {
        let meter = Meter::new(50.0).width(usize::MAX);
        let line = meter.plain();

        assert_eq!(meter.width, MAX_METER_WIDTH);
        assert_eq!(visible_len(&line), MAX_METER_WIDTH);
    }
}
