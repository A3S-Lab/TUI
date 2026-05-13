use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{visible_len, Color, Style};

pub struct StatusBar {
    left: String,
    center: String,
    right: String,
    fg: Color,
    bg: Color,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            left: String::new(),
            center: String::new(),
            right: String::new(),
            fg: Color::White,
            bg: Color::BrightBlack,
        }
    }

    pub fn left(mut self, s: impl Into<String>) -> Self {
        self.left = s.into();
        self
    }

    pub fn center(mut self, s: impl Into<String>) -> Self {
        self.center = s.into();
        self
    }

    pub fn right(mut self, s: impl Into<String>) -> Self {
        self.right = s.into();
        self
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg = color;
        self
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children: Vec<Element<Msg>> = Vec::new();

        if !self.left.is_empty() {
            children.push(Element::Text(TextElement::new(&self.left).fg(self.fg)));
        }

        children.push(Element::Spacer);

        if !self.center.is_empty() {
            children.push(Element::Text(TextElement::new(&self.center).fg(self.fg)));
            children.push(Element::Spacer);
        }

        if !self.right.is_empty() {
            children.push(Element::Text(TextElement::new(&self.right).fg(self.fg)));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .bg(self.bg)
                .children(children),
        )
    }

    pub fn view(&self, width: u16) -> String {
        let w = width as usize;
        let left_width = visible_len(&self.left);
        let center_width = visible_len(&self.center);
        let right_width = visible_len(&self.right);

        let mut line = String::new();
        line.push_str(&self.left);

        let space_for_center = w.saturating_sub(left_width + right_width);
        if center_width > 0 && space_for_center >= center_width {
            let center_start = (w.saturating_sub(center_width)) / 2;
            let pad_before = center_start.saturating_sub(left_width);
            line.push_str(&" ".repeat(pad_before));
            line.push_str(&self.center);
            let used = left_width + pad_before + center_width;
            let pad_after = w.saturating_sub(used + right_width);
            line.push_str(&" ".repeat(pad_after));
        } else {
            let middle_space = w.saturating_sub(left_width + right_width);
            line.push_str(&" ".repeat(middle_space));
        }

        line.push_str(&self.right);

        let total_vis = visible_len(&line);
        if total_vis < w {
            line.push_str(&" ".repeat(w - total_vis));
        }

        Style::new().fg(self.fg).bg(self.bg).render(&line)
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn empty_status_bar() {
        let sb = StatusBar::new();
        let view = sb.view(40);
        let plain = strip_ansi(&view);
        assert_eq!(plain.len(), 40);
    }

    #[test]
    fn left_content() {
        let sb = StatusBar::new().left("hello");
        let view = sb.view(40);
        let plain = strip_ansi(&view);
        assert!(plain.starts_with("hello"));
    }

    #[test]
    fn right_content() {
        let sb = StatusBar::new().right("end");
        let view = sb.view(40);
        let plain = strip_ansi(&view);
        assert!(plain.ends_with("end"));
    }

    #[test]
    fn left_and_right() {
        let sb = StatusBar::new().left("L").right("R");
        let view = sb.view(20);
        let plain = strip_ansi(&view);
        assert!(plain.starts_with('L'));
        assert!(plain.ends_with('R'));
        assert_eq!(plain.len(), 20);
    }

    #[test]
    fn element_produces_row() {
        let sb = StatusBar::new().left("a").right("b");
        let el: Element<()> = sb.element();
        match el {
            Element::Box(b) => {
                assert_eq!(b.style.flex_direction, crate::element::FlexDirection::Row);
            }
            _ => panic!("expected Box"),
        }
    }
}
