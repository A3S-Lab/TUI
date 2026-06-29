use crate::style::{visible_len, Color, Style};

const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

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
        self.width = width.max(1);
        self
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
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
            return self.empty.to_string().repeat(self.width);
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
            out = format!("{}{}", self.empty.to_string().repeat(self.width - len), out);
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
}
