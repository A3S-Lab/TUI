use crate::element::{Element, TextElement};
use crate::style::Color;

pub fn divider<Msg>() -> Element<Msg> {
    Element::Text(TextElement::new("─".repeat(200)).fg(Color::BrightBlack))
}

pub fn divider_with<Msg>(ch: &str, color: Color) -> Element<Msg> {
    Element::Text(TextElement::new(ch.repeat(200)).fg(color))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divider_creates_text() {
        let el: Element<()> = divider();
        match el {
            Element::Text(t) => assert!(t.content.contains('─')),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn divider_with_custom_char() {
        let el: Element<()> = divider_with("═", Color::Red);
        match el {
            Element::Text(t) => {
                assert!(t.content.contains('═'));
                assert_eq!(t.style.fg, Some(Color::Red));
            }
            _ => panic!("expected Text"),
        }
    }
}
