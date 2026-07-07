use crate::style::{repeat_visible_char, visible_len, Color, Style};

const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const MAX_SPARKLINE_WIDTH: usize = u16::MAX as usize;

#[derive(Debug, Clone)]
pub struct Sparkline {
    values: Vec<f64>,
    width: usize,
    min: Option<f64>,
    max: Option<f64>,
    fg: Color,
    empty: char,
}

impl Sparkline {
    pub fn new(values: impl IntoIterator<Item = f64>) -> Self {
        Self {
            values: values.into_iter().filter(|v| v.is_finite()).collect(),
            width: 10,
            min: Some(0.0),
            max: None,
            fg: Color::Cyan,
            empty: '·',
        }
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = width.clamp(1, MAX_SPARKLINE_WIDTH);
        self
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        match (min.is_finite(), max.is_finite()) {
            (true, true) if min <= max => {
                self.min = Some(min);
                self.max = Some(max);
            }
            (true, true) => {
                self.min = Some(max);
                self.max = Some(min);
            }
            (true, false) => {
                self.min = Some(min);
                self.max = None;
            }
            (false, true) => {
                self.min = None;
                self.max = Some(max);
            }
            (false, false) => {
                self.min = None;
                self.max = None;
            }
        }
        self
    }

    pub fn auto_range(mut self) -> Self {
        self.min = None;
        self.max = None;
        self
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    pub fn empty(mut self, ch: char) -> Self {
        self.empty = ch;
        self
    }

    pub fn view(&self) -> String {
        Style::new().fg(self.fg).render(&self.plain())
    }

    pub fn plain(&self) -> String {
        if self.width == 0 {
            return String::new();
        }
        if self.values.is_empty() {
            return repeat_visible_char(self.empty, self.width);
        }

        let values = self.window_values();
        let observed_min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let observed_max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min = self.min.unwrap_or(observed_min);
        let max = self.max.unwrap_or(observed_max).max(min);
        let span = (max - min).max(f64::EPSILON);

        let mut out = values
            .iter()
            .map(|value| {
                let normalized = ((*value - min) / span).clamp(0.0, 1.0);
                let idx = (normalized * (BARS.len() - 1) as f64).round() as usize;
                BARS[idx]
            })
            .collect::<String>();

        let len = visible_len(&out);
        if len < self.width {
            out = format!(
                "{}{}",
                repeat_visible_char(self.empty, self.width - len),
                out
            );
        }
        out
    }

    fn window_values(&self) -> Vec<f64> {
        let len = self.values.len();
        if len <= self.width {
            return self.values.clone();
        }
        self.values[len - self.width..].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn renders_empty_placeholder() {
        let line = Sparkline::new(Vec::<f64>::new()).width(4).plain();

        assert_eq!(line, "····");
    }

    #[test]
    fn custom_wide_empty_glyph_respects_display_width() {
        let empty = Sparkline::new(Vec::<f64>::new())
            .width(3)
            .empty('界')
            .plain();
        let padded = Sparkline::new([1.0]).width(4).empty('界').plain();

        assert_eq!(visible_len(&empty), 3);
        assert_eq!(empty, "界 ");
        assert_eq!(visible_len(&padded), 4);
        assert_eq!(padded, "界 █");
    }

    #[test]
    fn renders_fixed_width() {
        let line = Sparkline::new([0.0, 25.0, 50.0, 100.0])
            .width(4)
            .range(0.0, 100.0)
            .plain();

        assert_eq!(visible_len(&line), 4);
        assert!(line.ends_with('█'));
    }

    #[test]
    fn uses_recent_window() {
        let line = Sparkline::new([0.0, 0.0, 100.0]).width(2).plain();

        assert_eq!(visible_len(&line), 2);
        assert!(line.ends_with('█'));
    }

    #[test]
    fn view_is_styled() {
        let line = Sparkline::new([1.0]).width(1).view();

        assert_ne!(strip_ansi(&line), line);
    }

    #[test]
    fn non_finite_range_falls_back_to_observed_values() {
        let line = Sparkline::new([0.0, 50.0, 100.0])
            .width(3)
            .range(f64::NAN, f64::INFINITY)
            .plain();

        assert_eq!(visible_len(&line), 3);
        assert!(line.ends_with('█'));
    }

    #[test]
    fn reversed_range_bounds_are_sorted() {
        let line = Sparkline::new([0.0, 50.0, 100.0])
            .width(3)
            .range(100.0, 0.0)
            .plain();

        assert_eq!(line, "▁▅█");
    }

    #[test]
    fn oversized_width_is_clamped() {
        let sparkline = Sparkline::new(Vec::<f64>::new()).width(usize::MAX);
        let line = sparkline.plain();

        assert_eq!(sparkline.width, MAX_SPARKLINE_WIDTH);
        assert_eq!(visible_len(&line), MAX_SPARKLINE_WIDTH);
    }
}
