use crate::element::{BoxElement, BorderStyle, Element, FlexDirection, TextElement};
use crate::style::Color;

#[derive(Debug, Clone, Copy)]
pub enum AlertKind {
    Info,
    Success,
    Warning,
    Error,
}

pub struct Alert {
    kind: AlertKind,
    title: String,
    body: String,
}

impl Alert {
    pub fn new(kind: AlertKind, body: impl Into<String>) -> Self {
        Self {
            kind,
            title: String::new(),
            body: body.into(),
        }
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let (icon, color) = match self.kind {
            AlertKind::Info => ("ℹ", Color::Blue),
            AlertKind::Success => ("✓", Color::Green),
            AlertKind::Warning => ("⚠", Color::Yellow),
            AlertKind::Error => ("✗", Color::Red),
        };

        let mut children: Vec<Element<Msg>> = Vec::new();

        if !self.title.is_empty() {
            children.push(Element::Text(
                TextElement::new(format!("{} {}", icon, self.title)).bold().fg(color),
            ));
        } else {
            children.push(Element::Text(
                TextElement::new(icon).bold().fg(color),
            ));
        }

        children.push(Element::Text(TextElement::new(&self.body)));

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .border(BorderStyle::Rounded)
                .border_color(color)
                .padding(1)
                .children(children),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_without_title() {
        let alert = Alert::new(AlertKind::Info, "message");
        let el: Element<()> = alert.element();
        match el {
            Element::Box(b) => assert_eq!(b.children.len(), 2),
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn alert_with_title() {
        let alert = Alert::new(AlertKind::Success, "done").title("Status");
        let el: Element<()> = alert.element();
        match el {
            Element::Box(b) => assert_eq!(b.children.len(), 2),
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn alert_kinds() {
        for kind in [AlertKind::Info, AlertKind::Success, AlertKind::Warning, AlertKind::Error] {
            let alert = Alert::new(kind, "test");
            let _el: Element<()> = alert.element();
        }
    }
}
