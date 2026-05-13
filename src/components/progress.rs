use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{Color, Style};

pub struct Progress {
    value: f64,
    width: u16,
    filled_char: char,
    empty_char: char,
    filled_color: Color,
    empty_color: Color,
    show_percentage: bool,
}

impl Progress {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            width: 40,
            filled_char: '█',
            empty_char: '░',
            filled_color: Color::Green,
            empty_color: Color::BrightBlack,
            show_percentage: true,
        }
    }

    pub fn value(mut self, v: f64) -> Self {
        self.value = v.clamp(0.0, 1.0);
        self
    }

    pub fn set_value(&mut self, v: f64) {
        self.value = v.clamp(0.0, 1.0);
    }

    pub fn width(mut self, w: u16) -> Self {
        self.width = w;
        self
    }

    pub fn filled_color(mut self, color: Color) -> Self {
        self.filled_color = color;
        self
    }

    pub fn empty_color(mut self, color: Color) -> Self {
        self.empty_color = color;
        self
    }

    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let bar_width = if self.show_percentage {
            self.width.saturating_sub(5) as usize
        } else {
            self.width as usize
        };

        let filled_count = (self.value * bar_width as f64).round() as usize;
        let empty_count = bar_width.saturating_sub(filled_count);

        let filled_str = self.filled_char.to_string().repeat(filled_count);
        let empty_str = self.empty_char.to_string().repeat(empty_count);

        let mut children: Vec<Element<Msg>> = vec![
            Element::Text(TextElement::new(filled_str).fg(self.filled_color)),
            Element::Text(TextElement::new(empty_str).fg(self.empty_color)),
        ];

        if self.show_percentage {
            let pct = format!(" {:3.0}%", self.value * 100.0);
            children.push(Element::Text(TextElement::new(pct)));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }

    pub fn view(&self) -> String {
        let bar_width = if self.show_percentage {
            self.width.saturating_sub(5) as usize
        } else {
            self.width as usize
        };

        let filled_count = (self.value * bar_width as f64).round() as usize;
        let empty_count = bar_width.saturating_sub(filled_count);

        let filled = Style::new()
            .fg(self.filled_color)
            .render(&self.filled_char.to_string().repeat(filled_count));
        let empty = Style::new()
            .fg(self.empty_color)
            .render(&self.empty_char.to_string().repeat(empty_count));

        if self.show_percentage {
            let pct = format!("{:3.0}%", self.value * 100.0);
            format!("{}{} {}", filled, empty, pct)
        } else {
            format!("{}{}", filled, empty)
        }
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_value_is_zero() {
        let p = Progress::new();
        assert_eq!(p.value, 0.0);
    }

    #[test]
    fn value_clamps() {
        let p = Progress::new().value(1.5);
        assert_eq!(p.value, 1.0);
        let p2 = Progress::new().value(-0.5);
        assert_eq!(p2.value, 0.0);
    }

    #[test]
    fn set_value() {
        let mut p = Progress::new();
        p.set_value(0.75);
        assert_eq!(p.value, 0.75);
    }

    #[test]
    fn view_contains_percentage() {
        let p = Progress::new().value(0.5);
        let view = p.view();
        assert!(view.contains("50%"));
    }

    #[test]
    fn view_without_percentage() {
        let p = Progress::new().value(0.5).show_percentage(false);
        let view = p.view();
        assert!(!view.contains('%'));
    }

    #[test]
    fn element_produces_row() {
        let p = Progress::new().value(0.5);
        let el: Element<()> = p.element();
        match el {
            Element::Box(b) => {
                assert_eq!(b.style.flex_direction, FlexDirection::Row);
            }
            _ => panic!("expected Box"),
        }
    }
}
