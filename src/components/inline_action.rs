use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, Color, Style};

/// Inline action pill with optional muted detail text.
#[derive(Debug, Clone)]
pub struct InlineAction {
    label: String,
    icon: Option<String>,
    detail: Option<String>,
    fg: Color,
    bg: Color,
    detail_color: Color,
    bold: bool,
}

impl InlineAction {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            detail: None,
            fg: Color::BrightWhite,
            bg: Color::Cyan,
            detail_color: Color::BrightBlack,
            bold: true,
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        let detail = detail.into();
        self.detail = (!detail.trim().is_empty()).then_some(detail);
        self
    }

    pub fn colors(mut self, fg: Color, bg: Color) -> Self {
        self.fg = fg;
        self.bg = bg;
        self
    }

    pub fn detail_color(mut self, color: Color) -> Self {
        self.detail_color = color;
        self
    }

    pub fn bold(mut self, enabled: bool) -> Self {
        self.bold = enabled;
        self
    }

    pub fn view(&self) -> String {
        let action = self.action_text();
        let mut style = Style::new().fg(self.fg).bg(self.bg);
        if self.bold {
            style = style.bold();
        }
        let button = style.render(&action);
        match self
            .detail
            .as_deref()
            .map(str::trim)
            .filter(|detail| !detail.is_empty())
        {
            Some(detail) => format!(
                "{button} {}",
                Style::new().fg(self.detail_color).render(detail)
            ),
            None => button,
        }
    }

    pub fn view_with_width(&self, width: u16) -> String {
        fit_visible(&self.view(), width as usize)
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children = vec![Element::Text(self.action_element_text())];
        if let Some(detail) = self
            .detail
            .as_deref()
            .map(str::trim)
            .filter(|detail| !detail.is_empty())
        {
            children.push(Element::Text(TextElement::new(" ")));
            children.push(Element::Text(
                TextElement::new(detail).fg(self.detail_color),
            ));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }

    fn action_text(&self) -> String {
        match self
            .icon
            .as_deref()
            .map(str::trim)
            .filter(|icon| !icon.is_empty())
        {
            Some(icon) => format!(" {icon} {} ", self.label),
            None => format!(" {} ", self.label),
        }
    }

    fn action_element_text(&self) -> TextElement {
        let mut text = TextElement::new(self.action_text()).fg(self.fg).bg(self.bg);
        if self.bold {
            text = text.bold();
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn view_renders_action_and_detail() {
        let rendered = InlineAction::new("Open view")
            .icon("↗")
            .colors(Color::BrightWhite, Color::Blue)
            .detail("click or /view to open")
            .view();
        let plain = strip_ansi(&rendered);

        assert_eq!(plain, " ↗ Open view  click or /view to open");
        assert!(rendered.contains("\x1b["));
    }

    #[test]
    fn empty_detail_is_omitted() {
        let rendered = InlineAction::new("Run").detail("  ").view();

        assert_eq!(strip_ansi(&rendered), " Run ");
    }

    #[test]
    fn view_with_width_bounds_visible_length() {
        let rendered = InlineAction::new("Open view")
            .icon("↗")
            .detail("a very long detail")
            .view_with_width(12);

        assert_eq!(visible_len(&rendered), 12);
    }

    #[test]
    fn element_contains_button_and_detail_rows() {
        let el: Element<()> = InlineAction::new("Open view")
            .icon("↗")
            .detail("click")
            .element();

        let Element::Box(row) = el else {
            panic!("expected row");
        };
        assert_eq!(row.style.flex_direction, FlexDirection::Row);
        assert_eq!(row.children.len(), 3);
        assert_eq!(row.children[0].text_content(), Some(" ↗ Open view "));
        assert_eq!(row.children[2].text_content(), Some("click"));
    }
}
