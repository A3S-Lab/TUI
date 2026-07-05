use crate::element::{Element, TextElement};
use crate::style::{repeat_visible, Color, Style};

const DIVIDER_WIDTH: usize = 200;

pub fn divider<Msg>() -> Element<Msg> {
    Element::Text(TextElement::new(repeat_visible("─", DIVIDER_WIDTH)).fg(Color::BrightBlack))
}

pub fn divider_with<Msg>(ch: &str, color: Color) -> Element<Msg> {
    Element::Text(TextElement::new(repeat_visible(ch, DIVIDER_WIDTH)).fg(color))
}

pub fn divider_line(width: u16) -> String {
    divider_line_with(width, "─", Color::BrightBlack)
}

pub fn divider_line_with(width: u16, ch: &str, color: Color) -> String {
    Style::new()
        .fg(color)
        .render(&repeat_visible(ch, width as usize))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;
    use crate::style::visible_len;

    #[test]
    fn divider_creates_text() {
        let el: Element<()> = divider();
        match el {
            Element::Text(t) => {
                assert!(t.content.contains('─'));
                assert_eq!(visible_len(&t.content), DIVIDER_WIDTH);
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn divider_with_custom_char() {
        let el: Element<()> = divider_with("═", Color::Red);
        match el {
            Element::Text(t) => {
                assert!(t.content.contains('═'));
                assert_eq!(visible_len(&t.content), DIVIDER_WIDTH);
                assert_eq!(t.style.fg, Some(Color::Red));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn divider_with_wide_pattern_fills_visible_width() {
        let el: Element<()> = divider_with("界", Color::Cyan);

        match el {
            Element::Text(t) => {
                assert_eq!(visible_len(&t.content), DIVIDER_WIDTH);
                assert_eq!(t.content.chars().count(), DIVIDER_WIDTH / 2);
                assert_eq!(t.style.fg, Some(Color::Cyan));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn divider_with_zero_width_pattern_falls_back_to_spaces() {
        let el: Element<()> = divider_with("\u{301}", Color::Cyan);

        match el {
            Element::Text(t) => {
                assert_eq!(visible_len(&t.content), DIVIDER_WIDTH);
                assert!(t.content.chars().all(|ch| ch == ' '));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn divider_line_renders_styled_bounded_row() {
        let rendered = divider_line(12);
        let plain = strip_ansi(&rendered);

        assert_eq!(visible_len(&rendered), 12);
        assert_eq!(plain, "────────────");
        assert!(rendered.contains("\x1b["));
    }

    #[test]
    fn divider_line_with_custom_wide_pattern() {
        let rendered = divider_line_with(8, "界", Color::Cyan);
        let plain = strip_ansi(&rendered);

        assert_eq!(visible_len(&rendered), 8);
        assert_eq!(plain.chars().count(), 4);
        assert!(rendered.contains("\x1b["));
    }
}
