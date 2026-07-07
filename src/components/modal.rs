use crate::element::{BorderStyle as ElBorder, BoxElement, Element, FlexDirection, TextElement};
use crate::style::{
    fit_visible, split_nonempty_lines_preserving_trailing_blank, visible_len, Border, Color, Style,
};

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
        self.clamp_selected();
        self
    }

    pub fn selected(mut self, idx: usize) -> Self {
        self.selected = idx;
        if !self.options.is_empty() {
            self.clamp_selected();
        }
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
                    let current = self.selected.min(self.options.len() - 1);
                    self.selected = (current + 1) % self.options.len();
                }
                None
            }
            ModalMsg::Prev => {
                if !self.options.is_empty() {
                    let current = self.selected.min(self.options.len() - 1);
                    self.selected = current.checked_sub(1).unwrap_or(self.options.len() - 1);
                }
                None
            }
            ModalMsg::Select(idx) => {
                if idx < self.options.len() {
                    self.selected = idx;
                    Some(idx)
                } else {
                    None
                }
            }
            ModalMsg::Cancel => None,
        }
    }

    pub fn confirm(&self) -> usize {
        self.normalized_selected()
    }

    pub fn view(&self, screen_width: u16, screen_height: u16) -> String {
        let screen_width = screen_width as usize;
        let screen_height = screen_height as usize;
        if screen_width == 0 || screen_height == 0 {
            return String::new();
        }

        let content_width = self.compute_width();
        let modal_width = content_width.saturating_add(4).min(screen_width);

        let mut inner_lines = Vec::new();

        if !self.title.is_empty() {
            let title_styled = Style::new().bold().fg(self.title_color).render(&self.title);
            inner_lines.push(title_styled);
            inner_lines.push(String::new());
        }

        if !self.body.is_empty() {
            for line in split_nonempty_lines_preserving_trailing_blank(&self.body) {
                inner_lines.push(line.to_string());
            }
            inner_lines.push(String::new());
        }

        let selected = self.normalized_selected();
        for (i, opt) in self.options.iter().enumerate() {
            let prefix = if i == selected { "▸ " } else { "  " };
            let styled = if i == selected {
                Style::new()
                    .bold()
                    .fg(self.selected_color)
                    .render(&format!("{}{}", prefix, opt))
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

        let top_pad = screen_height.saturating_sub(box_height) / 2;
        let left_pad = screen_width.saturating_sub(modal_width) / 2;

        let mut output = Vec::new();

        for _ in 0..top_pad {
            output.push(String::new());
        }

        for line in &box_lines {
            output.push(format!("{}{}", " ".repeat(left_pad), line));
        }

        for _ in 0..screen_height.saturating_sub(top_pad + box_height) {
            output.push(String::new());
        }

        output.truncate(screen_height);
        while output.len() < screen_height {
            output.push(String::new());
        }

        output
            .into_iter()
            .map(|line| fit_visible(&line, screen_width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn compute_width(&self) -> usize {
        let mut max_width = visible_len(&self.title);
        for line in split_nonempty_lines_preserving_trailing_blank(&self.body) {
            max_width = max_width.max(visible_len(line));
        }
        for opt in &self.options {
            max_width = max_width.max(visible_len(opt) + 2);
        }
        max_width.max(20)
    }

    fn clamp_selected(&mut self) {
        self.selected = self.selected.min(self.options.len().saturating_sub(1));
    }

    fn normalized_selected(&self) -> usize {
        self.selected.min(self.options.len().saturating_sub(1))
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
            for line in split_nonempty_lines_preserving_trailing_blank(&self.body) {
                children.push(Element::Text(TextElement::new(line)));
            }
            children.push(Element::Text(TextElement::new("")));
        }

        let selected = self.normalized_selected();
        for (i, opt) in self.options.iter().enumerate() {
            let prefix = if i == selected { "▸ " } else { "  " };
            let text = format!("{}{}", prefix, opt);
            if i == selected {
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
        assert_eq!(modal.confirm(), 1);
    }

    #[test]
    fn modal_cancel_returns_none() {
        let mut modal = Modal::new().options(vec!["X"]);
        let result = modal.update(ModalMsg::Cancel);
        assert_eq!(result, None);
    }

    #[test]
    fn selected_index_is_clamped() {
        let modal = Modal::new().options(vec!["X", "Y"]).selected(99);

        assert_eq!(modal.confirm(), 1);
    }

    #[test]
    fn navigation_normalizes_stale_selection() {
        let mut modal = Modal::new()
            .selected(usize::MAX)
            .options(vec!["A", "B", "C"]);
        assert_eq!(modal.confirm(), 2);

        modal.update(ModalMsg::Next);
        assert_eq!(modal.confirm(), 0);

        let mut modal = Modal::new().options(vec!["A", "B", "C"]);
        modal.selected = usize::MAX;
        modal.update(ModalMsg::Prev);
        assert_eq!(modal.confirm(), 1);
    }

    #[test]
    fn rendering_normalizes_stale_selection() {
        let mut modal = Modal::new().options(vec!["A", "B"]);
        modal.selected = usize::MAX;

        let rendered = modal.view(30, 6);

        assert!(rendered.contains("▸ B"));
        assert!(!rendered.contains("▸ A"));

        let Element::Box(box_el) = modal.element::<()>() else {
            panic!("expected Box");
        };
        let Element::Text(last_option) = box_el.children.last().expect("expected last option")
        else {
            panic!("expected option text");
        };
        assert_eq!(last_option.content, "▸ B");
    }

    #[test]
    fn select_ignores_out_of_bounds_index() {
        let mut modal = Modal::new().options(vec!["X", "Y"]);

        assert_eq!(modal.update(ModalMsg::Select(99)), None);
        assert_eq!(modal.confirm(), 0);
    }

    #[test]
    fn modal_view_clamps_to_screen_size() {
        let rendered = Modal::new()
            .title("Confirm an unusually long operation title")
            .body("This body is also far longer than the available modal width.")
            .options(vec!["Accept the full operation", "Cancel"])
            .view(16, 5);

        assert_eq!(rendered.lines().count(), 5);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 16, "{line:?}");
        }
    }

    #[test]
    fn modal_view_returns_empty_for_zero_screen_dimensions() {
        let modal = Modal::new().title("Confirm").options(vec!["OK"]);

        assert_eq!(modal.view(0, 5), "");
        assert_eq!(modal.view(20, 0), "");
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

    #[test]
    fn modal_element_preserves_trailing_blank_body_line() {
        let modal = Modal::new().body("Line\n").options(vec!["OK"]);
        let el: Element<()> = modal.element();

        let Element::Box(box_el) = el else {
            panic!("expected Box");
        };
        let Element::Text(body) = &box_el.children[0] else {
            panic!("expected body text");
        };
        let Element::Text(blank_body) = &box_el.children[1] else {
            panic!("expected trailing blank body row");
        };
        let Element::Text(separator) = &box_el.children[2] else {
            panic!("expected separator row");
        };

        assert_eq!(body.content, "Line");
        assert_eq!(blank_body.content, "");
        assert_eq!(separator.content, "");
    }
}
