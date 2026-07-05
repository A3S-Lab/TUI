use crate::style::fit_visible;

const MAX_TEXT_OVERLAY_WIDTH: usize = u16::MAX as usize;

/// Where overlay rows are placed in a rendered text frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOverlayPosition {
    /// Place the first overlay row at the given row.
    StartAt(usize),
    /// Place the last overlay row at the given row.
    EndAt(usize),
    /// Place rows from the top of the frame.
    Top,
    /// Place rows so the last overlay row lands on the final frame row.
    Bottom,
    /// Place rows so `rows_below` frame rows remain below the overlay.
    AboveBottom { rows_below: usize },
}

/// Compose transient overlay rows into an already-rendered terminal frame.
///
/// This extracts the common string-rendering pattern used by command palettes,
/// file pickers, approval prompts, and side panels: render the base screen, then
/// replace a small row range with a menu or panel near the bottom.
#[derive(Debug, Clone)]
pub struct TextOverlay {
    rows: Vec<String>,
    position: TextOverlayPosition,
    width: Option<usize>,
}

impl TextOverlay {
    pub fn new(rows: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            rows: rows.into_iter().map(Into::into).collect(),
            position: TextOverlayPosition::Bottom,
            width: None,
        }
    }

    pub fn start_at(mut self, row: usize) -> Self {
        self.position = TextOverlayPosition::StartAt(row);
        self
    }

    pub fn end_at(mut self, row: usize) -> Self {
        self.position = TextOverlayPosition::EndAt(row);
        self
    }

    pub fn top(mut self) -> Self {
        self.position = TextOverlayPosition::Top;
        self
    }

    pub fn bottom(mut self) -> Self {
        self.position = TextOverlayPosition::Bottom;
        self
    }

    pub fn above_bottom(mut self, rows_below: usize) -> Self {
        self.position = TextOverlayPosition::AboveBottom { rows_below };
        self
    }

    /// Fit every overlay row to the provided display-column width.
    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width.min(MAX_TEXT_OVERLAY_WIDTH));
        self
    }

    pub fn apply(&self, frame: &str) -> String {
        if frame.is_empty() || self.rows.is_empty() {
            return frame.to_string();
        }

        let mut frame_rows: Vec<String> = frame.split('\n').map(str::to_string).collect();
        if frame_rows.is_empty() {
            return frame.to_string();
        }

        let start = self.start_row(frame_rows.len());
        for (idx, row) in self.rows.iter().enumerate() {
            let target = start + idx;
            if target >= frame_rows.len() {
                break;
            }
            frame_rows[target] = self.render_row(row);
        }

        frame_rows.join("\n")
    }

    fn start_row(&self, frame_height: usize) -> usize {
        match self.position {
            TextOverlayPosition::StartAt(row) => row,
            TextOverlayPosition::EndAt(row) => {
                row.saturating_sub(self.rows.len().saturating_sub(1))
            }
            TextOverlayPosition::Top => 0,
            TextOverlayPosition::Bottom => frame_height.saturating_sub(self.rows.len()),
            TextOverlayPosition::AboveBottom { rows_below } => {
                let end_row = frame_height.saturating_sub(1).saturating_sub(rows_below);
                end_row.saturating_sub(self.rows.len().saturating_sub(1))
            }
        }
    }

    fn render_row(&self, row: &str) -> String {
        self.width
            .map(|width| fit_visible(row, width))
            .unwrap_or_else(|| row.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len, Color, Style};

    fn frame(rows: usize) -> String {
        (0..rows)
            .map(|idx| format!("row {idx}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn overlays_rows_ending_at_requested_row() {
        let rendered = TextOverlay::new(["one", "two"]).end_at(3).apply(&frame(5));
        let rows = rendered.lines().collect::<Vec<_>>();

        assert_eq!(rows, vec!["row 0", "row 1", "one", "two", "row 4"]);
    }

    #[test]
    fn overlays_rows_above_reserved_bottom_area() {
        let rendered = TextOverlay::new(["menu a", "menu b", "menu c"])
            .above_bottom(5)
            .apply(&frame(10));
        let rows = rendered.lines().collect::<Vec<_>>();

        assert_eq!(rows[2], "menu a");
        assert_eq!(rows[3], "menu b");
        assert_eq!(rows[4], "menu c");
        assert_eq!(rows[5], "row 5");
    }

    #[test]
    fn overlays_from_top_or_start_row() {
        let top = TextOverlay::new(["title"]).top().apply(&frame(3));
        assert_eq!(top.lines().next(), Some("title"));

        let start = TextOverlay::new(["panel"]).start_at(1).apply(&frame(3));
        assert_eq!(start.lines().nth(1), Some("panel"));
    }

    #[test]
    fn bottom_overlay_preserves_frame_height() {
        let rendered = TextOverlay::new(["a", "b", "c"]).bottom().apply(&frame(2));
        let rows = rendered.lines().collect::<Vec<_>>();

        assert_eq!(rows, vec!["a", "b"]);
    }

    #[test]
    fn empty_overlay_is_noop() {
        let base = frame(3);
        let rows: Vec<String> = Vec::new();

        assert_eq!(TextOverlay::new(rows).apply(&base), base);
    }

    #[test]
    fn empty_frame_stays_empty() {
        let rendered = TextOverlay::new(["menu"]).top().apply("");

        assert_eq!(rendered, "");
    }

    #[test]
    fn width_fits_styled_and_wide_rows() {
        let styled = Style::new().fg(Color::Cyan).render("中文abcdef");
        let rendered = TextOverlay::new([styled]).width(8).top().apply(&frame(2));
        let first = rendered.lines().next().unwrap();

        assert_eq!(visible_len(first), 8);
        assert!(strip_ansi(first).contains("中文"));
        assert!(first.contains('\u{1b}'));
    }

    #[test]
    fn oversized_width_is_clamped() {
        let overlay = TextOverlay::new(["menu"]).width(usize::MAX);
        let rendered = overlay.apply(&frame(1));
        let first = rendered.lines().next().unwrap();

        assert_eq!(overlay.width, Some(MAX_TEXT_OVERLAY_WIDTH));
        assert_eq!(visible_len(first), MAX_TEXT_OVERLAY_WIDTH);
    }
}
