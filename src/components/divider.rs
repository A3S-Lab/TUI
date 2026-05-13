use crate::element::{Element, TextElement};
use crate::style::Color;

pub fn divider<Msg>() -> Element<Msg> {
    Element::Text(TextElement::new("─".repeat(200)).fg(Color::BrightBlack))
}

pub fn divider_with<Msg>(ch: &str, color: Color) -> Element<Msg> {
    Element::Text(TextElement::new(ch.repeat(200)).fg(color))
}
