use crate::style::{truncate_visible, visible_len, Color, Style};

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
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = max.max(f64::EPSILON);
        self
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = width.max(1);
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

    pub fn view(&self) -> String {
        let value = if self.value.is_finite() {
            self.value
        } else {
            0.0
        };
        let ratio = (value / self.max).clamp(0.0, 1.0);
        let value_label = if (self.max - 100.0).abs() < f64::EPSILON {
            format!("{value:>5.1}%")
        } else {
            format!("{value:>6.1}")
        };
        let prefix = if self.label.is_empty() {
            format!("{value_label} ")
        } else {
            format!("{} {value_label} ", self.label)
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
                .render(&self.fill.to_string().repeat(filled)),
            Style::new()
                .fg(self.empty_fg)
                .render(&self.empty.to_string().repeat(empty))
        );

        format!("{prefix}{bar}")
    }

    pub fn plain(&self) -> String {
        crate::style::strip_ansi(&self.view())
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
}
