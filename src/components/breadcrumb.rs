use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::Color;

/// A breadcrumb navigation component.
///
/// Displays a path of items separated by a configurable separator,
/// with the last item highlighted as the current location.
pub struct Breadcrumb {
    items: Vec<String>,
    separator: String,
    active_color: Color,
    inactive_color: Color,
    separator_color: Color,
}

impl Breadcrumb {
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(|i| i.into()).collect(),
            separator: " › ".to_string(),
            active_color: Color::White,
            inactive_color: Color::BrightBlack,
            separator_color: Color::BrightBlack,
        }
    }

    pub fn separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }

    pub fn active_color(mut self, color: Color) -> Self {
        self.active_color = color;
        self
    }

    pub fn inactive_color(mut self, color: Color) -> Self {
        self.inactive_color = color;
        self
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children: Vec<Element<Msg>> = Vec::new();
        let last_idx = self.items.len().saturating_sub(1);

        for (i, item) in self.items.iter().enumerate() {
            let is_last = i == last_idx;

            if is_last {
                children.push(Element::Text(
                    TextElement::new(item.as_str()).bold().fg(self.active_color),
                ));
            } else {
                children.push(Element::Text(
                    TextElement::new(item.as_str()).fg(self.inactive_color),
                ));
                children.push(Element::Text(
                    TextElement::new(&self.separator).fg(self.separator_color),
                ));
            }
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_item() {
        let bc = Breadcrumb::new(vec!["Home"]);
        let el: Element<()> = bc.element();
        match el {
            Element::Box(b) => assert_eq!(b.children.len(), 1),
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn multiple_items_have_separators() {
        let bc = Breadcrumb::new(vec!["Home", "Docs", "API"]);
        let el: Element<()> = bc.element();
        match el {
            Element::Box(b) => {
                // 3 items + 2 separators = 5 children
                assert_eq!(b.children.len(), 5);
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn custom_separator() {
        let bc = Breadcrumb::new(vec!["a", "b"]).separator(" / ");
        let el: Element<()> = bc.element();
        match el {
            Element::Box(b) => match &b.children[1] {
                Element::Text(t) => assert_eq!(t.content, " / "),
                _ => panic!("expected separator text"),
            },
            _ => panic!("expected Box"),
        }
    }
}
