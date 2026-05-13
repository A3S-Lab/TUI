use crate::element::{BoxElement, BorderStyle, Element, TextElement};
use crate::style::Color;

pub struct Badge {
    label: String,
    color: Color,
}

impl Badge {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            color: Color::Cyan,
        }
    }

    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        Element::Box(
            BoxElement::new()
                .border(BorderStyle::Rounded)
                .border_color(self.color)
                .child(Element::Text(TextElement::new(&self.label).fg(self.color).bold())),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_default_color() {
        let badge = Badge::new("test");
        assert_eq!(badge.color, Color::Cyan);
    }

    #[test]
    fn badge_custom_color() {
        let badge = Badge::new("OK").color(Color::Green);
        assert_eq!(badge.color, Color::Green);
    }

    #[test]
    fn badge_element() {
        let badge = Badge::new("v1.0");
        let el: Element<()> = badge.element();
        match el {
            Element::Box(b) => assert_eq!(b.children.len(), 1),
            _ => panic!("expected Box"),
        }
    }
}
