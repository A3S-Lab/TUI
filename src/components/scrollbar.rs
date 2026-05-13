use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::Color;

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
}
