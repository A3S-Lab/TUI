use crate::style::{
    ansi_escape_sequence_end, fit_visible, next_display_cell_boundary, osc8_link_target,
    truncate_visible, visible_len,
};

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
/// replace a small row range with a menu or panel near the bottom. By default it
/// replaces complete rows; [`Self::at_column`] and [`Self::centered`] preserve
/// the base content outside each overlay row's occupied columns.
#[derive(Debug, Clone)]
pub struct TextOverlay {
    rows: Vec<String>,
    position: TextOverlayPosition,
    horizontal_position: TextOverlayHorizontalPosition,
    width: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextOverlayHorizontalPosition {
    ReplaceRow,
    AtColumn(usize),
    Centered,
}

impl TextOverlay {
    pub fn new(rows: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            rows: rows.into_iter().map(Into::into).collect(),
            position: TextOverlayPosition::Bottom,
            horizontal_position: TextOverlayHorizontalPosition::ReplaceRow,
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

    /// Set the display-column width used by this overlay.
    ///
    /// In the default whole-row mode and in [`Self::at_column`] mode, every
    /// overlay row is fitted to this width. In [`Self::centered`] mode, this is
    /// the width of the container in which the widest overlay row is centered;
    /// individual rows are not padded to the container width.
    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width.min(MAX_TEXT_OVERLAY_WIDTH));
        self
    }

    /// Overlay text starting at `column`, measured in terminal display columns.
    ///
    /// Only the columns occupied by an overlay row are replaced. Base content
    /// before and after that span, including its ANSI styling, is preserved.
    pub fn at_column(mut self, column: usize) -> Self {
        self.horizontal_position =
            TextOverlayHorizontalPosition::AtColumn(column.min(MAX_TEXT_OVERLAY_WIDTH));
        self
    }

    /// Center the overlay's widest row while preserving base content around it.
    ///
    /// When [`Self::width`] is set, that width is the centering container. When
    /// it is not set, the widest base-frame row supplies the container width.
    /// All overlay rows share the same starting column.
    pub fn centered(mut self) -> Self {
        self.horizontal_position = TextOverlayHorizontalPosition::Centered;
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
        let centered_layout = self.centered_layout(&frame_rows);
        for (idx, row) in self.rows.iter().enumerate() {
            let target = start.saturating_add(idx);
            if target >= frame_rows.len() {
                break;
            }
            let rendered_row = self.render_row(row, centered_layout.map(|layout| layout.width));
            frame_rows[target] = match self.horizontal_position {
                TextOverlayHorizontalPosition::ReplaceRow => rendered_row,
                TextOverlayHorizontalPosition::AtColumn(column) => {
                    overlay_visible_columns(&frame_rows[target], &rendered_row, column)
                }
                TextOverlayHorizontalPosition::Centered => overlay_visible_columns(
                    &frame_rows[target],
                    &rendered_row,
                    centered_layout.map(|layout| layout.start).unwrap_or(0),
                ),
            };
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

    fn centered_layout(&self, frame_rows: &[String]) -> Option<CenteredLayout> {
        if self.horizontal_position != TextOverlayHorizontalPosition::Centered {
            return None;
        }

        let width = self.width.unwrap_or_else(|| {
            frame_rows
                .iter()
                .map(|row| visible_len(row))
                .max()
                .unwrap_or(0)
        });
        let overlay_width = self
            .rows
            .iter()
            .map(|row| visible_len(&truncate_visible(row, width)))
            .max()
            .unwrap_or(0);

        Some(CenteredLayout {
            start: width.saturating_sub(overlay_width) / 2,
            width,
        })
    }

    fn render_row(&self, row: &str, centered_width: Option<usize>) -> String {
        if let Some(width) = centered_width {
            truncate_visible(row, width)
        } else {
            self.width
                .map(|width| fit_visible(row, width))
                .unwrap_or_else(|| row.to_string())
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CenteredLayout {
    start: usize,
    width: usize,
}

fn overlay_visible_columns(base: &str, overlay: &str, start: usize) -> String {
    let overlay_width = visible_len(overlay);
    if overlay_width == 0 {
        return base.to_string();
    }

    let base_width = visible_len(base);
    let prefix_width = start.min(base_width);
    let overlay_end = start.saturating_add(overlay_width);
    let suffix_start = overlay_end.min(base_width);

    let mut composed = styled_visible_slice(base, 0, prefix_width);
    if start > base_width {
        composed.push_str(&" ".repeat(start - base_width));
    }
    composed.push_str(&isolate_ansi(overlay));
    composed.push_str(&styled_visible_slice(base, suffix_start, base_width));
    composed
}

/// Slice a styled row by display columns and make the result ANSI-independent.
///
/// A wide grapheme cluster that straddles a slice edge cannot be represented
/// partially in a terminal cell, so the intersecting columns are emitted as
/// styled spaces. Emoji sequences and combining marks stay intact.
fn styled_visible_slice(value: &str, from: usize, to: usize) -> String {
    if from >= to {
        return String::new();
    }

    let mut out = String::new();
    let mut ansi_state = AnsiReplay::default();
    let mut emitted_sgr = false;
    let mut emitted_hyperlink = false;
    let mut started = false;
    let mut last_cell_included = false;
    let mut column = 0usize;
    let mut index = 0usize;

    while index < value.len() {
        if let Some(end) = ansi_escape_sequence_end(value, index) {
            let sequence = &value[index..end];
            if started && (column < to || last_cell_included) {
                out.push_str(sequence);
                note_emitted_ansi(sequence, &mut emitted_sgr, &mut emitted_hyperlink);
            }
            ansi_state.observe(sequence);
            index = end;
            continue;
        }

        let Some((end, cell_width)) = next_display_cell_boundary(value, index) else {
            break;
        };
        let cell = &value[index..end];
        index = end;

        if cell_width == 0 {
            if last_cell_included {
                out.push_str(cell);
            }
            continue;
        }
        if column >= to {
            break;
        }

        let next_column = column.saturating_add(cell_width);
        let overlap_start = column.max(from);
        let overlap_end = next_column.min(to);
        let overlaps = overlap_start < overlap_end;
        let fully_included = column >= from && next_column <= to;

        if overlaps {
            if !started {
                ansi_state.write_prefix(&mut out, &mut emitted_sgr, &mut emitted_hyperlink);
                started = true;
            }
            if fully_included {
                out.push_str(cell);
            } else {
                out.push_str(&" ".repeat(overlap_end - overlap_start));
            }
        }

        last_cell_included = fully_included;
        column = next_column;
    }

    if emitted_sgr {
        out.push_str("\x1b[0m");
    }
    if emitted_hyperlink {
        out.push_str("\x1b]8;;\x1b\\");
    }
    out
}

#[derive(Debug, Default)]
struct AnsiReplay {
    sgr: String,
    hyperlink: Option<String>,
}

impl AnsiReplay {
    fn observe(&mut self, sequence: &str) {
        if let Some(params) = sgr_params(sequence) {
            if params
                .split(';')
                .next()
                .is_some_and(|value| value.is_empty() || value == "0")
            {
                self.sgr.clear();
            }
            self.sgr.push_str(sequence);
        }
        if let Some(target) = osc8_link_target(sequence) {
            self.hyperlink = (!target.is_empty()).then(|| sequence.to_string());
        }
    }

    fn write_prefix(&self, out: &mut String, emitted_sgr: &mut bool, emitted_hyperlink: &mut bool) {
        if !self.sgr.is_empty() {
            out.push_str(&self.sgr);
            *emitted_sgr = true;
        }
        if let Some(hyperlink) = &self.hyperlink {
            out.push_str(hyperlink);
            *emitted_hyperlink = true;
        }
    }
}

fn isolate_ansi(value: &str) -> String {
    let mut saw_sgr = false;
    let mut hyperlink_open = false;
    let mut index = 0usize;

    while index < value.len() {
        if let Some(end) = ansi_escape_sequence_end(value, index) {
            let sequence = &value[index..end];
            if sgr_params(sequence).is_some() {
                saw_sgr = true;
            }
            if let Some(target) = osc8_link_target(sequence) {
                hyperlink_open = !target.is_empty();
            }
            index = end;
            continue;
        }
        let ch = value[index..].chars().next().unwrap_or_default();
        index += ch.len_utf8();
    }

    let mut isolated = value.to_string();
    if saw_sgr {
        isolated.push_str("\x1b[0m");
    }
    if hyperlink_open {
        isolated.push_str("\x1b]8;;\x1b\\");
    }
    isolated
}

fn note_emitted_ansi(sequence: &str, emitted_sgr: &mut bool, hyperlink_open: &mut bool) {
    if sgr_params(sequence).is_some() {
        *emitted_sgr = true;
    }
    if let Some(target) = osc8_link_target(sequence) {
        *hyperlink_open = !target.is_empty();
    }
}

fn sgr_params(sequence: &str) -> Option<&str> {
    sequence
        .strip_prefix("\x1b[")
        .and_then(|value| value.strip_suffix('m'))
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

    #[test]
    fn overlays_at_display_column_without_replacing_row_edges() {
        let rendered = TextOverlay::new(["OK"])
            .at_column(4)
            .top()
            .apply("leftMIDDLEright");

        assert_eq!(rendered, "leftOKDDLEright");
    }

    #[test]
    fn centered_uses_width_as_container_and_aligns_to_widest_row() {
        let rendered = TextOverlay::new(["one", "12345"])
            .top()
            .width(16)
            .centered()
            .apply("0123456789ABCDEF\nabcdefghijklmnop");
        let rows = rendered.lines().collect::<Vec<_>>();

        assert_eq!(rows, vec!["01234one89ABCDEF", "abcde12345klmnop"]);
    }

    #[test]
    fn column_overlay_preserves_ansi_styles_on_both_sides() {
        let base = Style::new().fg(Color::Red).render("leftMIDDLEright");
        let overlay = Style::new().fg(Color::Cyan).render("OK");
        let rendered = TextOverlay::new([overlay]).at_column(4).top().apply(&base);

        assert_eq!(strip_ansi(&rendered), "leftOKDDLEright");
        assert!(rendered.contains("\x1b[31mleft"), "{rendered:?}");
        assert!(rendered.contains("\x1b[36mOK"), "{rendered:?}");
        assert!(rendered.contains("\x1b[31mDDLEright"), "{rendered:?}");
    }

    #[test]
    fn column_overlay_replays_compound_sgr_state() {
        let base = "\x1b[1m\x1b[38;2;255;0;0mabcdef\x1b[0m";
        let rendered = TextOverlay::new(["X"]).at_column(2).top().apply(base);

        assert_eq!(strip_ansi(&rendered), "abXdef");
        assert_eq!(rendered.matches("\x1b[1m").count(), 2, "{rendered:?}");
        assert_eq!(
            rendered.matches("\x1b[38;2;255;0;0m").count(),
            2,
            "{rendered:?}"
        );
    }

    #[test]
    fn column_overlay_closes_and_reopens_base_hyperlinks() {
        let open = "\x1b]8;;https://example.com\x1b\\";
        let close = "\x1b]8;;\x1b\\";
        let base = format!("{open}abcdef{close}");
        let rendered = TextOverlay::new(["X"]).at_column(2).top().apply(&base);

        assert_eq!(strip_ansi(&rendered), "abXdef");
        assert_eq!(rendered.matches(open).count(), 2, "{rendered:?}");
    }

    #[test]
    fn column_overlay_clears_straddled_wide_base_cells() {
        let right_half = TextOverlay::new(["X"]).at_column(3).top().apply("ab界cd");
        let left_half = TextOverlay::new(["X"]).at_column(2).top().apply("ab界cd");

        assert_eq!(right_half, "ab Xcd");
        assert_eq!(left_half, "abX cd");
        assert_eq!(visible_len(&right_half), 6);
        assert_eq!(visible_len(&left_half), 6);
    }

    #[test]
    fn column_overlay_keeps_combining_marks_with_their_base_glyphs() {
        let replaced = TextOverlay::new(["o\u{301}"])
            .at_column(1)
            .top()
            .apply("ae\u{301}bc");
        let preserved = TextOverlay::new(["X"])
            .at_column(2)
            .top()
            .apply("ae\u{301}bc");

        assert_eq!(replaced, "ao\u{301}bc");
        assert_eq!(preserved, "ae\u{301}Xc");
    }

    #[test]
    fn column_overlay_pads_to_columns_beyond_short_base_rows() {
        let rendered = TextOverlay::new(["ok"]).at_column(5).top().apply("hi");

        assert_eq!(rendered, "hi   ok");
    }

    #[test]
    fn width_still_fits_rows_in_at_column_mode() {
        let rendered = TextOverlay::new(["X"])
            .at_column(2)
            .width(4)
            .top()
            .apply("abcdefghij");

        assert_eq!(rendered, "abX   ghij");
    }

    fn assert_centered_overlay_preserves_wide_grapheme(grapheme: &str) {
        let base = format!("ab{grapheme}cd");
        assert_eq!(visible_len(&base), 6, "unexpected fixture width: {base:?}");

        let rendered = TextOverlay::new(["X"])
            .top()
            .width(6)
            .centered()
            .apply(&base);

        assert_eq!(rendered, "abX cd");
        assert_eq!(visible_len(&rendered), 6);
    }

    #[test]
    fn centered_overlay_preserves_zwj_emoji_cell_boundaries() {
        assert_centered_overlay_preserves_wide_grapheme("🧑\u{200d}🤝\u{200d}🧑");
    }

    #[test]
    fn centered_overlay_preserves_emoji_modifier_cell_boundaries() {
        assert_centered_overlay_preserves_wide_grapheme("👍🏽");
    }

    #[test]
    fn centered_overlay_preserves_flag_cell_boundaries() {
        assert_centered_overlay_preserves_wide_grapheme("🇺🇳");
    }

    #[test]
    fn centered_overlay_preserves_variation_selector_cell_boundaries() {
        assert_centered_overlay_preserves_wide_grapheme("☀\u{fe0f}");
    }
}
