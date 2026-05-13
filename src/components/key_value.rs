use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::Color;

/// A key-value pair display component.
///
/// Renders a list of labeled values in a consistent format.
pub struct KeyValue {
    pairs: Vec<(String, String)>,
    key_color: Color,
    value_color: Color,
    separator: String,
}

impl KeyValue {
    pub fn new() -> Self {
        Self {
            pairs: Vec::new(),
            key_color: Color::BrightBlack,
            value_color: Color::White,
            separator: ": ".to_string(),
        }
    }

    pub fn pair(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.pairs.push((key.into(), value.into()));
        self
    }

    pub fn add(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.pairs.push((key.into(), value.into()));
    }

    pub fn key_color(mut self, color: Color) -> Self {
        self.key_color = color;
        self
    }

    pub fn value_color(mut self, color: Color) -> Self {
        self.value_color = color;
        self
    }

    pub fn separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let children: Vec<Element<Msg>> = self
            .pairs
            .iter()
            .map(|(key, value)| {
                Element::Box(
                    BoxElement::new()
                        .direction(FlexDirection::Row)
                        .child(Element::Text(TextElement::new(key.as_str()).fg(self.key_color)))
                        .child(Element::Text(TextElement::new(&self.separator)))
                        .child(Element::Text(TextElement::new(value.as_str()).fg(self.value_color))),
                )
            })
            .collect();

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }
}

impl Default for KeyValue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_kv() {
        let kv = KeyValue::new();
        let el: Element<()> = kv.element();
        match el {
            Element::Box(b) => assert_eq!(b.children.len(), 0),
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn multiple_pairs() {
        let kv = KeyValue::new()
            .pair("Name", "a3s-tui")
            .pair("Version", "0.1.0")
            .pair("License", "MIT");
        let el: Element<()> = kv.element();
        match el {
            Element::Box(b) => assert_eq!(b.children.len(), 3),
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn add_method() {
        let mut kv = KeyValue::new();
        kv.add("key", "value");
        let el: Element<()> = kv.element();
        match el {
            Element::Box(b) => assert_eq!(b.children.len(), 1),
            _ => panic!("expected Box"),
        }
    }
}
