use crate::element::{BorderStyle, BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, Color, Style};

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
    color: Option<Color>,
}

impl Alert {
    pub fn new(kind: AlertKind, body: impl Into<String>) -> Self {
        Self {
            kind,
            title: String::new(),
            body: body.into(),
            color: None,
        }
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn view(&self) -> String {
        let (icon, color) = self.presentation();
        Style::new().fg(color).render(&self.line_text(icon))
    }

    pub fn view_with_width(&self, width: u16) -> String {
        fit_visible(&self.view(), width as usize)
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let (icon, color) = self.presentation();

        let mut children: Vec<Element<Msg>> = Vec::new();

        if !self.title.is_empty() {
            children.push(Element::Text(
                TextElement::new(format!("{} {}", icon, self.title))
                    .bold()
                    .fg(color),
            ));
        } else {
            children.push(Element::Text(TextElement::new(icon).bold().fg(color)));
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

    fn presentation(&self) -> (&'static str, Color) {
        let (icon, default_color) = match self.kind {
            AlertKind::Info => ("ℹ", Color::Blue),
            AlertKind::Success => ("✓", Color::Green),
            AlertKind::Warning => ("⚠", Color::Yellow),
            AlertKind::Error => ("✗", Color::Red),
        };
        (icon, self.color.unwrap_or(default_color))
    }

    fn line_text(&self, icon: &str) -> String {
        let title = self.title.trim();
        let body = self.body.trim();
        match (!title.is_empty(), !body.is_empty()) {
            (true, true) => format!("{icon} {title}: {body}"),
            (true, false) => format!("{icon} {title}"),
            (false, true) => format!("{icon} {body}"),
            (false, false) => icon.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

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
        for kind in [
            AlertKind::Info,
            AlertKind::Success,
            AlertKind::Warning,
            AlertKind::Error,
        ] {
            let alert = Alert::new(kind, "test");
            let _el: Element<()> = alert.element();
        }
    }

    #[test]
    fn alert_view_renders_icon_title_and_body() {
        let rendered = Alert::new(AlertKind::Warning, "needs login")
            .title("OS")
            .view();

        assert_eq!(strip_ansi(&rendered), "⚠ OS: needs login");
        assert!(rendered.contains("\x1b[33m"));
    }

    #[test]
    fn alert_view_without_title_renders_body() {
        let rendered = Alert::new(AlertKind::Error, "failed").view();

        assert_eq!(strip_ansi(&rendered), "✗ failed");
    }

    #[test]
    fn alert_view_uses_custom_color() {
        let rendered = Alert::new(AlertKind::Info, "custom")
            .color(Color::Rgb(1, 2, 3))
            .view();

        assert_eq!(strip_ansi(&rendered), "ℹ custom");
        assert!(rendered.contains("\x1b[38;2;1;2;3m"));
    }

    #[test]
    fn alert_view_with_width_bounds_visible_length() {
        let rendered = Alert::new(AlertKind::Warning, "a long warning").view_with_width(8);

        assert_eq!(visible_len(&rendered), 8);
    }
}
