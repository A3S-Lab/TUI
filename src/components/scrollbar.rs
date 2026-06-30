use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{pad_visible, Color, Style};

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

        let thumb_size = ((self.visible as f64 / self.total as f64) * height as f64)
            .ceil()
            .max(1.0) as usize;

        let max_offset = self.total.saturating_sub(self.visible);
        let thumb_pos = if max_offset == 0 {
            0
        } else {
            ((self.offset as f64 / max_offset as f64) * (height - thumb_size) as f64).round()
                as usize
        };

        (thumb_pos, thumb_size)
    }

    /// Render as a vertical Element column.
    pub fn element<Msg>(&self, height: usize) -> Element<Msg> {
        let (thumb_pos, thumb_size) = self.thumb_range(height);

        let children: Vec<Element<Msg>> = (0..height)
            .map(|i| {
                if i >= thumb_pos && i < thumb_pos + thumb_size {
                    Element::Text(
                        TextElement::new(self.thumb_char.to_string()).fg(self.thumb_color),
                    )
                } else {
                    Element::Text(
                        TextElement::new(self.track_char.to_string()).fg(self.track_color),
                    )
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
                if i >= thumb_pos && i < thumb_pos + thumb_size {
                    self.thumb_char
                } else {
                    self.track_char
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
                if i >= thumb_pos && i < thumb_pos + thumb_size {
                    Style::new()
                        .fg(self.thumb_color)
                        .render(&self.thumb_char.to_string())
                } else {
                    Style::new()
                        .fg(self.track_color)
                        .render(&self.track_char.to_string())
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Append this scrollbar as a one-column gutter to a rendered text view.
    pub fn append_to_view(&self, view: &str, inner_width: usize) -> String {
        let rows = view.split('\n').collect::<Vec<_>>();
        let bar = self.styled_view(rows.len());
        rows.into_iter()
            .zip(bar.lines())
            .map(|(row, bar)| format!("{}{}", pad_visible(row, inner_width), bar))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let plain = crate::style::strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("short   "));
        assert!(rows[1].starts_with("中文    "));
        assert!(rendered.contains("\x1b["));
    }
}
