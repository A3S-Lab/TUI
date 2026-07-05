use crate::components::Sparkline;
use crate::style::{fit_visible, right_visible, truncate_visible, visible_len, Color};

const MAX_METRIC_TREND_WIDTH: usize = u16::MAX as usize;
const MAX_METRIC_TREND_SPARKLINE_WIDTH: usize = u16::MAX as usize;

#[derive(Debug, Clone)]
pub struct MetricTrend {
    value: Option<f64>,
    history: Vec<f64>,
    width: usize,
    trend_width: usize,
    min: f64,
    max: f64,
    fg: Color,
    missing: String,
}

impl MetricTrend {
    pub fn new(value: Option<f64>, history: impl IntoIterator<Item = f64>) -> Self {
        Self {
            value: value.filter(|value| value.is_finite()),
            history: history
                .into_iter()
                .filter(|value| value.is_finite())
                .collect(),
            width: 15,
            trend_width: 8,
            min: 0.0,
            max: 100.0,
            fg: Color::Cyan,
            missing: "-".into(),
        }
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = width.clamp(1, MAX_METRIC_TREND_WIDTH);
        self
    }

    pub fn trend_width(mut self, width: usize) -> Self {
        self.trend_width = width.clamp(1, MAX_METRIC_TREND_SPARKLINE_WIDTH);
        self
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        if min.is_finite() && max.is_finite() {
            if min <= max {
                self.min = min;
                self.max = max;
            } else {
                self.min = max;
                self.max = min;
            }
        }
        self
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    pub fn missing(mut self, missing: impl Into<String>) -> Self {
        self.missing = missing.into();
        self
    }

    pub fn view(&self) -> String {
        let label = self.value_label();
        if self.width <= visible_len(&label) {
            return truncate_visible(&label, self.width);
        }

        let available = self.width - visible_len(&label) - 1;
        if available == 0 {
            return truncate_visible(&label, self.width);
        }
        let trend_width = self.trend_width.min(available);
        let trend = Sparkline::new(self.history.iter().copied())
            .width(trend_width)
            .range(self.min, self.max)
            .fg(self.fg)
            .view();
        let cell = format!("{label} {trend}");

        fit_visible(&cell, self.width)
    }

    pub fn plain(&self) -> String {
        crate::style::strip_ansi(&self.view())
    }

    fn value_label(&self) -> String {
        self.value
            .map(|value| format!("{value:>5.1}%"))
            .unwrap_or_else(|| right_visible(&self.missing, 6))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_metric_and_trend_at_fixed_width() {
        let cell = MetricTrend::new(Some(12.5), [0.0, 10.0, 20.0])
            .width(15)
            .plain();

        assert_eq!(visible_len(&cell), 15);
        assert!(cell.starts_with(" 12.5% "));
    }

    #[test]
    fn renders_missing_metric_with_placeholder_trend() {
        let cell = MetricTrend::new(None, Vec::<f64>::new()).width(15).plain();

        assert_eq!(visible_len(&cell), 15);
        assert!(cell.starts_with("     - "));
        assert!(cell.contains("····"));
    }

    #[test]
    fn missing_metric_placeholder_uses_display_width() {
        let cell = MetricTrend::new(None, Vec::<f64>::new())
            .missing("暂无")
            .width(15)
            .plain();

        assert_eq!(visible_len(&cell), 15);
        assert!(cell.starts_with("  暂无 "));
        assert!(cell.contains("····"));
    }

    #[test]
    fn truncates_to_narrow_width() {
        let cell = MetricTrend::new(Some(123.4), [100.0]).width(5).plain();

        assert_eq!(visible_len(&cell), 5);
    }

    #[test]
    fn ignores_non_finite_range_bounds() {
        let cell = MetricTrend::new(Some(50.0), [0.0, 50.0, 100.0])
            .range(f64::NAN, f64::INFINITY)
            .width(15)
            .plain();

        assert_eq!(visible_len(&cell), 15);
        assert!(cell.starts_with(" 50.0% "));
        assert!(cell.ends_with('█'));
    }

    #[test]
    fn reversed_range_bounds_are_sorted() {
        let cell = MetricTrend::new(Some(50.0), [0.0, 50.0, 100.0])
            .range(100.0, 0.0)
            .width(10)
            .trend_width(3)
            .plain();

        assert!(cell.ends_with("▁▅█"));
    }

    #[test]
    fn oversized_widths_are_clamped() {
        let trend = MetricTrend::new(Some(50.0), [0.0, 50.0, 100.0])
            .width(usize::MAX)
            .trend_width(usize::MAX);
        let cell = trend.plain();

        assert_eq!(trend.width, MAX_METRIC_TREND_WIDTH);
        assert_eq!(trend.trend_width, MAX_METRIC_TREND_SPARKLINE_WIDTH);
        assert_eq!(visible_len(&cell), MAX_METRIC_TREND_WIDTH);
    }
}
