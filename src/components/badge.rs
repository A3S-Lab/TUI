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
