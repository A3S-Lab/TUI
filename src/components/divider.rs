use crate::element::{Element, TextElement};
use crate::style::{repeat_visible, Color};

const DIVIDER_WIDTH: usize = 200;

pub fn divider<Msg>() -> Element<Msg> {
    Element::Text(TextElement::new(repeat_visible("─", DIVIDER_WIDTH)).fg(Color::BrightBlack))
}

pub fn divider_with<Msg>(ch: &str, color: Color) -> Element<Msg> {
    Element::Text(TextElement::new(repeat_visible(ch, DIVIDER_WIDTH)).fg(color))
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
