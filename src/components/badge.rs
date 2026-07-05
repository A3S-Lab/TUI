use crate::element::{BorderStyle, BoxElement, Element, TextElement};
use crate::style::{Color, Style};

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

    pub fn view(&self) -> String {
        Style::new()
            .fg(self.color)
            .bold()
            .render(&format!("[{}]", self.label))
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        Element::Box(
            BoxElement::new()
                .border(BorderStyle::Rounded)
                .border_color(self.color)
                .child(Element::Text(
                    TextElement::new(&self.label).fg(self.color).bold(),
                )),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

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

    #[test]
    fn badge_view_renders_colored_label() {
        let rendered = Badge::new("sem").color(Color::Green).view();

        assert_eq!(strip_ansi(&rendered), "[sem]");
        assert!(rendered.contains("\x1b[1;32m[sem]\x1b[0m"));
    }
}
