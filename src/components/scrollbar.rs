use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{pad_visible, repeat_visible_char, Color, Style};

/// A vertical scrollbar indicator.
///
/// Displays a track with a thumb that indicates the current scroll position.
/// Pair with `Viewport` or `List` to show scroll state.
pub struct Scrollbar {
    total: usize,
    visible: usize,
    offset: usize,
    track_char: char,
    thumb_char: char,
    track_color: Color,
    thumb_color: Color,
    hide_when_not_overflowing: bool,
}

impl Scrollbar {
    pub fn new(total: usize, visible: usize, offset: usize) -> Self {
        Self {
            total,
            visible,
            offset,
            track_char: '│',
            thumb_char: '█',
            track_color: Color::BrightBlack,
            thumb_color: Color::White,
            hide_when_not_overflowing: false,
        }
    }

    /// Build a scrollbar from a 0-100 scroll percentage.
    pub fn from_scroll_percent(total: usize, visible: usize, scroll_percent: u8) -> Self {
        let max_offset = total.saturating_sub(visible);
        let offset = if max_offset == 0 {
            0
        } else {
            ((max_offset as u128 * scroll_percent.min(100) as u128) / 100) as usize
        };

        Self::new(total, visible, offset)
    }

    pub fn track_char(mut self, ch: char) -> Self {
        self.track_char = ch;
        self
    }

    pub fn thumb_char(mut self, ch: char) -> Self {
        self.thumb_char = ch;
        self
    }

    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = color;
        self
    }

    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = color;
        self
    }

    /// Render a blank gutter when the content fits in the viewport.
    pub fn hide_when_not_overflowing(mut self, enabled: bool) -> Self {
        self.hide_when_not_overflowing = enabled;
        self
    }

    pub fn has_overflow(&self) -> bool {
        self.total > self.visible && self.visible > 0
    }

    /// Compute thumb position and size for the given track height.
    fn thumb_range(&self, height: usize) -> (usize, usize) {
        if self.total <= self.visible || height == 0 {
            return (0, height);
        }

        let thumb_size =
            ceil_ratio_to_usize(self.visible as u128 * height as u128, self.total as u128).max(1);
        let thumb_size = thumb_size.min(height);

        let max_offset = self.total.saturating_sub(self.visible);
        let scrollable_track = height.saturating_sub(thumb_size);
        let thumb_pos = if max_offset == 0 {
            0
        } else {
            let offset = self.offset.min(max_offset);
            rounded_ratio_to_usize(
                offset as u128 * scrollable_track as u128,
                max_offset as u128,
            )
            .min(scrollable_track)
        };

        (thumb_pos, thumb_size)
    }

    /// Render as a vertical Element column.
    pub fn element<Msg>(&self, height: usize) -> Element<Msg> {
        let (thumb_pos, thumb_size) = self.thumb_range(height);

        let children: Vec<Element<Msg>> = (0..height)
            .map(|i| {
                if i >= thumb_pos && i < thumb_pos.saturating_add(thumb_size) {
                    Element::Text(TextElement::new(self.thumb_cell()).fg(self.thumb_color))
                } else {
                    Element::Text(TextElement::new(self.track_cell()).fg(self.track_color))
                }
            })
            .collect();

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    /// Render as a single-line string (for string-based rendering).
    pub fn view(&self, height: usize) -> String {
        if self.hide_when_not_overflowing && !self.has_overflow() {
            return " ".repeat(height);
        }

        let (thumb_pos, thumb_size) = self.thumb_range(height);

        (0..height)
            .map(|i| {
                if i >= thumb_pos && i < thumb_pos.saturating_add(thumb_size) {
                    self.thumb_cell()
                } else {
                    self.track_cell()
                }
            })
            .collect()
    }

    /// Render the vertical scrollbar as newline-separated styled cells.
    pub fn styled_view(&self, height: usize) -> String {
        if self.hide_when_not_overflowing && !self.has_overflow() {
            return (0..height)
                .map(|_| " ".to_string())
                .collect::<Vec<_>>()
                .join("\n");
        }

        let (thumb_pos, thumb_size) = self.thumb_range(height);
        (0..height)
            .map(|i| {
                if i >= thumb_pos && i < thumb_pos.saturating_add(thumb_size) {
                    Style::new().fg(self.thumb_color).render(&self.thumb_cell())
                } else {
                    Style::new().fg(self.track_color).render(&self.track_cell())
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Append this scrollbar as a one-column gutter to a rendered text view.
    pub fn append_to_view(&self, view: &str, inner_width: usize) -> String {
        if view.is_empty() {
            return String::new();
        }

        let rows = view.split('\n').collect::<Vec<_>>();
        let bar = self.styled_view(rows.len());
        rows.into_iter()
            .zip(bar.lines())
            .map(|(row, bar)| format!("{}{}", pad_visible(row, inner_width), bar))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn track_cell(&self) -> String {
        repeat_visible_char(self.track_char, 1)
    }

    fn thumb_cell(&self) -> String {
        repeat_visible_char(self.thumb_char, 1)
    }
}

fn ceil_ratio_to_usize(numerator: u128, denominator: u128) -> usize {
    debug_assert!(denominator > 0);

    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let value = quotient + u128::from(remainder > 0);

    debug_assert!(value <= usize::MAX as u128);
    value as usize
}

fn rounded_ratio_to_usize(numerator: u128, denominator: u128) -> usize {
    debug_assert!(denominator > 0);

    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let value = quotient + u128::from(remainder.saturating_mul(2) >= denominator);

    debug_assert!(value <= usize::MAX as u128);
    value as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn full_content_visible_fills_track() {
        let sb = Scrollbar::new(10, 10, 0);
        let (pos, size) = sb.thumb_range(10);
        assert_eq!(pos, 0);
        assert_eq!(size, 10);
    }

    #[test]
    fn half_content_visible() {
        let sb = Scrollbar::new(20, 10, 0);
        let (pos, size) = sb.thumb_range(10);
        assert_eq!(pos, 0);
        assert_eq!(size, 5);
    }

    #[test]
    fn scrolled_to_bottom() {
        let sb = Scrollbar::new(20, 10, 10);
        let (pos, size) = sb.thumb_range(10);
        assert_eq!(size, 5);
        assert_eq!(pos, 5);
    }

    #[test]
    fn from_scroll_percent_maps_percent_to_offset() {
        let top = Scrollbar::from_scroll_percent(20, 10, 0);
        let bottom = Scrollbar::from_scroll_percent(20, 10, 100);
        let clamped = Scrollbar::from_scroll_percent(20, 10, 200);

        assert_eq!(top.offset, 0);
        assert_eq!(bottom.offset, 10);
        assert_eq!(clamped.offset, 10);
    }

    #[test]
    fn oversized_offset_clamps_thumb_to_track() {
        let sb = Scrollbar::new(20, 10, usize::MAX);
        let (pos, size) = sb.thumb_range(10);

        assert_eq!(size, 5);
        assert_eq!(pos, 5);
        assert_eq!(sb.view(10).chars().last(), Some('█'));
    }

    #[test]
    fn huge_offsets_do_not_round_past_half_track() {
        let offset = usize::MAX / 2 - 511;
        let sb = Scrollbar::new(usize::MAX, 1, offset);
        let (pos, size) = sb.thumb_range(2);

        assert_eq!(size, 1);
        assert_eq!(pos, 0);
    }

    #[test]
    fn scrolled_to_middle() {
        let sb = Scrollbar::new(20, 10, 5);
        let (pos, size) = sb.thumb_range(10);
        assert_eq!(size, 5);
        assert!(pos > 0 && pos < 5);
    }

    #[test]
    fn minimum_thumb_size_is_one() {
        let sb = Scrollbar::new(1000, 1, 0);
        let (_, size) = sb.thumb_range(10);
        assert_eq!(size, 1);
    }

    #[test]
    fn view_renders_correct_length() {
        let sb = Scrollbar::new(20, 10, 0);
        let view = sb.view(10);
        assert_eq!(view.chars().count(), 10);
    }

    #[test]
    fn custom_wide_glyphs_render_one_column_cells() {
        let sb = Scrollbar::new(20, 5, 8).track_char('界').thumb_char('好');
        let view = sb.view(5);
        let styled = sb.styled_view(5);

        assert_eq!(visible_len(&view), 5);
        assert_eq!(view, "     ");
        for row in strip_ansi(&styled).lines() {
            assert_eq!(visible_len(row), 1, "{row:?}");
        }
    }

    #[test]
    fn element_glyphs_render_one_column_cells() {
        let element: Element<()> = Scrollbar::new(20, 5, 8)
            .track_char('界')
            .thumb_char('\u{301}')
            .element(5);

        let Element::Box(column) = element else {
            panic!("expected column");
        };

        for child in column.children {
            let Element::Text(text) = child else {
                panic!("expected text cell");
            };
            assert_eq!(visible_len(&text.content), 1);
            assert_eq!(text.content, " ");
        }
    }

    #[test]
    fn hide_when_not_overflowing_renders_blank_gutter() {
        let sb = Scrollbar::new(3, 5, 0).hide_when_not_overflowing(true);

        assert_eq!(sb.view(5), "     ");
        assert_eq!(sb.styled_view(3), " \n \n ");
    }

    #[test]
    fn styled_view_contains_ansi_for_track_and_thumb() {
        let sb = Scrollbar::new(20, 5, 10)
            .track_color(Color::BrightBlack)
            .thumb_color(Color::Cyan);
        let rendered = sb.styled_view(5);

        assert!(rendered.contains("\x1b["));
        assert_eq!(rendered.lines().count(), 5);
    }

    #[test]
    fn append_to_view_pads_rows_and_adds_gutter() {
        let view = "short\n中文";
        let rendered = Scrollbar::new(20, 2, 3)
            .thumb_color(Color::Cyan)
            .append_to_view(view, 8);
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("short   "));
        assert!(rows[1].starts_with("中文    "));
        assert!(rendered.contains("\x1b["));
    }

    #[test]
    fn append_to_empty_view_stays_empty() {
        let rendered = Scrollbar::new(20, 2, 3).append_to_view("", 8);

        assert_eq!(rendered, "");
    }

    #[test]
    fn append_to_view_keeps_wide_custom_glyph_gutter_to_one_column() {
        let rendered = Scrollbar::new(20, 2, 1)
            .track_char('界')
            .thumb_char('好')
            .append_to_view("short\n中文", 8);
        let plain = strip_ansi(&rendered);

        for row in plain.lines() {
            assert_eq!(visible_len(row), 9, "{row:?}");
        }
    }
}
