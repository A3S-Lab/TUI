use crate::element::{BoxElement, BorderStyle as ElBorder, Element, FlexDirection, TextElement};
use crate::style::{visible_len, Border, Color, Style};

pub struct Modal {
    title: String,
    body: String,
    options: Vec<String>,
    selected: usize,
    border: Border,
    border_color: Color,
    title_color: Color,
    selected_color: Color,
}

#[derive(Debug, Clone)]
pub enum ModalMsg {
    Next,
    Prev,
    Select(usize),
    Cancel,
}

impl Modal {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            body: String::new(),
            options: Vec::new(),
            selected: 0,
            border: Border::Rounded,
            border_color: Color::BrightBlue,
            title_color: Color::BrightWhite,
            selected_color: Color::BrightCyan,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    pub fn options(mut self, options: Vec<impl Into<String>>) -> Self {
        self.options = options.into_iter().map(|o| o.into()).collect();
        self
    }

    pub fn selected(mut self, idx: usize) -> Self {
        self.selected = idx;
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn update(&mut self, msg: ModalMsg) -> Option<usize> {
        match msg {
            ModalMsg::Next => {
                if !self.options.is_empty() {
                    self.selected = (self.selected + 1) % self.options.len();
                }
                None
            }
            ModalMsg::Prev => {
                if !self.options.is_empty() {
                    self.selected = self.selected.checked_sub(1).unwrap_or(self.options.len() - 1);
                }
                None
            }
            ModalMsg::Select(idx) => Some(idx),
            ModalMsg::Cancel => None,
        }
    }

    pub fn confirm(&self) -> usize {
        self.selected
    }

    pub fn view(&self, screen_width: u16, screen_height: u16) -> String {
        let content_width = self.compute_width();
        let modal_width = content_width + 4;

        let mut inner_lines = Vec::new();

        if !self.title.is_empty() {
            let title_styled = Style::new()
                .bold()
                .fg(self.title_color)
                .render(&self.title);
            inner_lines.push(title_styled);
            inner_lines.push(String::new());
        }

        if !self.body.is_empty() {
            for line in self.body.lines() {
                inner_lines.push(line.to_string());
            }
            inner_lines.push(String::new());
        }

        for (i, opt) in self.options.iter().enumerate() {
            let prefix = if i == self.selected { "▸ " } else { "  " };
            let styled = if i == self.selected {
                Style::new().bold().fg(self.selected_color).render(&format!("{}{}", prefix, opt))
            } else {
                format!("{}{}", prefix, opt)
            };
            inner_lines.push(styled);
        }

        let box_style = Style::new()
            .border(self.border)
            .border_fg(self.border_color)
            .padding(1, 2)
            .width(modal_width as u16);

        let box_content = inner_lines.join("\n");
        let rendered_box = box_style.render(&box_content);

        let box_lines: Vec<&str> = rendered_box.lines().collect();
        let box_height = box_lines.len();

        let top_pad = (screen_height as usize).saturating_sub(box_height) / 2;
        let left_pad = (screen_width as usize).saturating_sub(modal_width) / 2;

        let mut output = Vec::new();

        for _ in 0..top_pad {
            output.push(String::new());
        }

        for line in &box_lines {
            output.push(format!("{}{}", " ".repeat(left_pad), line));
        }

        for _ in 0..(screen_height as usize).saturating_sub(top_pad + box_height) {
            output.push(String::new());
        }

        output.join("\n")
    }

    fn compute_width(&self) -> usize {
        let mut max_width = visible_len(&self.title);
        for line in self.body.lines() {
            max_width = max_width.max(visible_len(line));
        }
        for opt in &self.options {
            max_width = max_width.max(visible_len(opt) + 2);
        }
        max_width.max(20)
    }
}

impl Default for Modal {
    fn default() -> Self {
        Self::new()
    }
}

impl Modal {
    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children: Vec<Element<Msg>> = Vec::new();

        if !self.title.is_empty() {
            children.push(Element::Text(
                TextElement::new(&self.title).bold().fg(self.title_color),
            ));
            children.push(Element::Text(TextElement::new("")));
        }

        if !self.body.is_empty() {
            for line in self.body.lines() {
                children.push(Element::Text(TextElement::new(line)));
            }
            children.push(Element::Text(TextElement::new("")));
        }

        for (i, opt) in self.options.iter().enumerate() {
            let prefix = if i == self.selected { "▸ " } else { "  " };
            let text = format!("{}{}", prefix, opt);
            if i == self.selected {
                children.push(Element::Text(
                    TextElement::new(text).bold().fg(self.selected_color),
                ));
            } else {
                children.push(Element::Text(TextElement::new(text)));
            }
        }

        let border = match self.border {
            Border::Rounded => ElBorder::Rounded,
            Border::Single => ElBorder::Single,
            Border::Double => ElBorder::Double,
            Border::Thick => ElBorder::Thick,
            _ => ElBorder::Rounded,
        };

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .border(border)
                .border_color(self.border_color)
                .padding(1)
                .children(children),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modal_update_next() {
        let mut modal = Modal::new().options(vec!["A", "B", "C"]);
        modal.update(ModalMsg::Next);
        assert_eq!(modal.confirm(), 1);
        modal.update(ModalMsg::Next);
        assert_eq!(modal.confirm(), 2);
        modal.update(ModalMsg::Next);
        assert_eq!(modal.confirm(), 0); // wraps
    }

    #[test]
    fn modal_update_prev() {
        let mut modal = Modal::new().options(vec!["A", "B", "C"]);
        modal.update(ModalMsg::Prev);
        assert_eq!(modal.confirm(), 2); // wraps to end
    }

    #[test]
    fn modal_select_returns_index() {
        let mut modal = Modal::new().options(vec!["X", "Y"]);
        let result = modal.update(ModalMsg::Select(1));
        assert_eq!(result, Some(1));
    }

    #[test]
    fn modal_cancel_returns_none() {
        let mut modal = Modal::new().options(vec!["X"]);
        let result = modal.update(ModalMsg::Cancel);
        assert_eq!(result, None);
    }

    #[test]
    fn modal_element_has_content() {
        let modal = Modal::new()
            .title("Confirm")
            .body("Are you sure?")
            .options(vec!["Yes", "No"]);
        let el: Element<()> = modal.element();
        match el {
            Element::Box(b) => {
                // title + empty + body + empty + 2 options = 6
                assert!(b.children.len() >= 5);
            }
            _ => panic!("expected Box"),
        }
    }
}
