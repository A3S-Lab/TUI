use crate::element::{BoxElement, Element, TextElement};
use crate::style::{next_display_cell_boundary, strip_ansi, visible_len, Color, Style};

const MAX_CYCLE_GAP: usize = isize::MAX as usize - 1;

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
        if spread.is_finite() {
            self.spread = spread.max(0.1);
        }
        self
    }

    pub fn cycle_gap(mut self, gap: usize) -> Self {
        self.cycle_gap = gap.min(MAX_CYCLE_GAP);
        self
    }

    pub fn speed_divisor(mut self, divisor: usize) -> Self {
        self.speed_divisor = divisor.max(1);
        self
    }

    pub fn bold_threshold(mut self, threshold: f32) -> Self {
        if threshold.is_finite() {
            self.bold_threshold = threshold.clamp(0.0, 1.0);
        }
        self
    }

    pub fn view(&self) -> String {
        self.styled_glyphs()
            .into_iter()
            .map(|glyph| {
                let mut style = Style::new().fg(glyph.color);
                if glyph.bold {
                    style = style.bold();
                }
                style.render(&glyph.text)
            })
            .collect()
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let glyphs = self.styled_glyphs();
        if glyphs.is_empty() {
            return Element::Text(TextElement::new(""));
        }

        Element::Box(
            BoxElement::row().children(
                glyphs
                    .into_iter()
                    .map(|glyph| {
                        let mut text = TextElement::new(glyph.text).fg(glyph.color);
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

    fn styled_glyphs(&self) -> Vec<ShimmerGlyph> {
        let plain = self.plain();
        let glyphs = display_glyphs(&plain);
        if glyphs.is_empty() {
            return Vec::new();
        }

        let span = glyphs
            .len()
            .saturating_add(self.cycle_gap)
            .min(MAX_CYCLE_GAP);
        let head = ((self.phase / self.speed_divisor) % span) as isize;

        glyphs
            .into_iter()
            .enumerate()
            .map(|(idx, text)| {
                let distance = (head - idx as isize).abs() as f32;
                let intensity = (1.0 - distance / self.spread).clamp(0.0, 1.0);
                ShimmerGlyph {
                    text,
                    color: mix_color(self.base_color, self.highlight_color, intensity),
                    bold: intensity > self.bold_threshold,
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShimmerGlyph {
    text: String,
    color: Color,
    bold: bool,
}

fn display_glyphs(value: &str) -> Vec<String> {
    let mut index = 0;
    let mut glyphs = Vec::new();
    while let Some((end, _)) = next_display_cell_boundary(value, index) {
        glyphs.push(value[index..end].to_string());
        index = end;
    }
    glyphs
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
    fn groups_combining_marks_with_base_glyph() {
        let shimmer = ShimmerText::new("e\u{301}x")
            .phase(0)
            .speed_divisor(1)
            .colors(Color::Rgb(0, 0, 0), Color::Rgb(100, 0, 0));
        let rendered = shimmer.view();
        let highlighted = Style::new()
            .fg(Color::Rgb(100, 0, 0))
            .bold()
            .render("e\u{301}");

        assert_eq!(strip_ansi(&rendered), "e\u{301}x");
        assert_eq!(shimmer.visible_width(), 2);
        assert!(rendered.contains(&highlighted), "{rendered:?}");

        let element: Element<()> = shimmer.element();
        match element {
            Element::Box(row) => {
                assert_eq!(row.children.len(), 2);
                match &row.children[0] {
                    Element::Text(text) => {
                        assert_eq!(text.content, "e\u{301}");
                        assert_eq!(text.style.fg, Some(Color::Rgb(100, 0, 0)));
                        assert!(text.style.bold);
                    }
                    _ => panic!("expected text element"),
                }
                match &row.children[1] {
                    Element::Text(text) => assert_eq!(text.content, "x"),
                    _ => panic!("expected text element"),
                }
            }
            _ => panic!("expected row element"),
        }
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

    #[test]
    fn ignores_non_finite_animation_settings() {
        let shimmer = ShimmerText::new("go")
            .spread(f32::INFINITY)
            .bold_threshold(f32::NAN);

        assert_eq!(shimmer.spread, 5.0);
        assert_eq!(shimmer.bold_threshold, 0.65);
        assert_eq!(strip_ansi(&shimmer.view()), "go");
    }

    #[test]
    fn clamps_oversized_cycle_gap() {
        let shimmer = ShimmerText::new("go").cycle_gap(usize::MAX);

        assert_eq!(shimmer.cycle_gap, MAX_CYCLE_GAP);
        assert_eq!(strip_ansi(&shimmer.view()), "go");
    }

    #[test]
    fn oversized_phase_wraps_within_cycle() {
        let rendered = ShimmerText::new("go")
            .phase(usize::MAX)
            .cycle_gap(1)
            .speed_divisor(1)
            .view();
        let expected = ShimmerText::new("go")
            .phase(usize::MAX % 3)
            .cycle_gap(1)
            .speed_divisor(1)
            .view();

        assert_eq!(rendered, expected);
        assert_eq!(strip_ansi(&rendered), "go");
    }
}
