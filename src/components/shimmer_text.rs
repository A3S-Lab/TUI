use crate::element::{BoxElement, Element, TextElement};
use crate::style::{strip_ansi, visible_len, Color, Style};

/// Animated text with a soft highlight moving across its glyphs.
///
/// This is useful for activity lines such as "Working..." where a spinner alone
/// is too small to signal ongoing progress. Input is treated as plain text:
/// existing ANSI sequences are stripped before rendering.
#[derive(Debug, Clone)]
pub struct ShimmerText {
    text: String,
    phase: usize,
    base_color: Color,
    highlight_color: Color,
    spread: f32,
    cycle_gap: usize,
    speed_divisor: usize,
    bold_threshold: f32,
}

impl ShimmerText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            phase: 0,
            base_color: Color::Rgb(37, 99, 235),
            highlight_color: Color::Rgb(228, 236, 255),
            spread: 5.0,
            cycle_gap: 12,
            speed_divisor: 3,
            bold_threshold: 0.65,
        }
    }

    pub fn phase(mut self, phase: usize) -> Self {
        self.phase = phase;
        self
    }

    pub fn set_phase(&mut self, phase: usize) {
        self.phase = phase;
    }

    pub fn tick(&mut self) {
        self.phase = self.phase.saturating_add(1);
    }

    pub fn colors(mut self, base: Color, highlight: Color) -> Self {
        self.base_color = base;
        self.highlight_color = highlight;
        self
    }

    pub fn spread(mut self, spread: f32) -> Self {
        self.spread = spread.max(0.1);
        self
    }

    pub fn cycle_gap(mut self, gap: usize) -> Self {
        self.cycle_gap = gap;
        self
    }

    pub fn speed_divisor(mut self, divisor: usize) -> Self {
        self.speed_divisor = divisor.max(1);
        self
    }

    pub fn bold_threshold(mut self, threshold: f32) -> Self {
        self.bold_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn view(&self) -> String {
        self.styled_chars()
            .into_iter()
            .map(|glyph| {
                let mut style = Style::new().fg(glyph.color);
                if glyph.bold {
                    style = style.bold();
                }
                style.render(&glyph.ch.to_string())
            })
            .collect()
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let glyphs = self.styled_chars();
        if glyphs.is_empty() {
            return Element::Text(TextElement::new(""));
        }

        Element::Box(
            BoxElement::row().children(
                glyphs
                    .into_iter()
                    .map(|glyph| {
                        let mut text = TextElement::new(glyph.ch.to_string()).fg(glyph.color);
                        if glyph.bold {
                            text = text.bold();
                        }
                        Element::Text(text)
                    })
                    .collect(),
            ),
        )
    }

    pub fn plain(&self) -> String {
        strip_ansi(&self.text)
    }

    pub fn visible_width(&self) -> usize {
        visible_len(&self.plain())
    }

    fn styled_chars(&self) -> Vec<ShimmerGlyph> {
        let chars: Vec<char> = self.plain().chars().collect();
        if chars.is_empty() {
            return Vec::new();
        }

        let span = (chars.len() + self.cycle_gap) as isize;
        let head = (self.phase / self.speed_divisor) as isize % span;

        chars
            .into_iter()
            .enumerate()
            .map(|(idx, ch)| {
                let distance = (head - idx as isize).abs() as f32;
                let intensity = (1.0 - distance / self.spread).clamp(0.0, 1.0);
                ShimmerGlyph {
                    ch,
                    color: mix_color(self.base_color, self.highlight_color, intensity),
                    bold: intensity > self.bold_threshold,
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShimmerGlyph {
    ch: char,
    color: Color,
    bold: bool,
}

fn mix_color(base: Color, highlight: Color, intensity: f32) -> Color {
    match (base, highlight) {
        (Color::Rgb(br, bg, bb), Color::Rgb(hr, hg, hb)) => {
            let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * intensity).round() as u8;
            Color::Rgb(lerp(br, hr), lerp(bg, hg), lerp(bb, hb))
        }
        _ if intensity > 0.0 => highlight,
        _ => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn empty_text_renders_empty() {
        let shimmer = ShimmerText::new("");

        assert_eq!(shimmer.view(), "");
        assert_eq!(shimmer.visible_width(), 0);
    }

    #[test]
    fn view_preserves_plain_text_and_adds_ansi() {
        let shimmer = ShimmerText::new("Working")
            .phase(0)
            .colors(Color::Rgb(0, 0, 0), Color::Rgb(100, 0, 0))
            .speed_divisor(1);
        let rendered = shimmer.view();

        assert_eq!(strip_ansi(&rendered), "Working");
        assert!(rendered.contains("\x1b[1;38;2;100;0;0mW\x1b[0m"));
    }

    #[test]
    fn phase_moves_highlight_across_text() {
        let first = ShimmerText::new("abc")
            .phase(0)
            .speed_divisor(1)
            .colors(Color::Rgb(0, 0, 0), Color::Rgb(100, 0, 0))
            .view();
        let second = ShimmerText::new("abc")
            .phase(1)
            .speed_divisor(1)
            .colors(Color::Rgb(0, 0, 0), Color::Rgb(100, 0, 0))
            .view();

        assert_ne!(first, second);
        assert!(second.contains("\x1b[1;38;2;100;0;0mb\x1b[0m"));
    }

    #[test]
    fn strips_existing_ansi_from_input() {
        let styled = Style::new().fg(Color::Red).render("ok");
        let shimmer = ShimmerText::new(styled);

        assert_eq!(shimmer.plain(), "ok");
        assert_eq!(strip_ansi(&shimmer.view()), "ok");
    }

    #[test]
    fn counts_cjk_display_width() {
        let shimmer = ShimmerText::new("工作中");

        assert_eq!(shimmer.visible_width(), 6);
        assert_eq!(strip_ansi(&shimmer.view()), "工作中");
    }

    #[test]
    fn element_uses_structured_text_segments() {
        let element: Element<()> = ShimmerText::new("go")
            .phase(0)
            .speed_divisor(1)
            .colors(Color::Rgb(0, 0, 0), Color::Rgb(100, 0, 0))
            .element();

        match element {
            Element::Box(row) => {
                assert_eq!(row.children.len(), 2);
                match &row.children[0] {
                    Element::Text(text) => {
                        assert_eq!(text.content, "g");
                        assert_eq!(text.style.fg, Some(Color::Rgb(100, 0, 0)));
                        assert!(text.style.bold);
                    }
                    _ => panic!("expected text element"),
                }
            }
            _ => panic!("expected row element"),
        }
    }
}
