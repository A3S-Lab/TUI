use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, repeat_visible_char, Color, Style};

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
        self.value = Self::normalize_value(v);
        self
    }

    pub fn set_value(&mut self, v: f64) {
        self.value = Self::normalize_value(v);
    }

    pub fn width(mut self, w: u16) -> Self {
        self.width = w;
        self
    }

    pub fn filled_char(mut self, ch: char) -> Self {
        self.filled_char = ch;
        self
    }

    pub fn empty_char(mut self, ch: char) -> Self {
        self.empty_char = ch;
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
        let bar_width = self.bar_width();
        let value = Self::normalize_value(self.value);

        let filled_count = (value * bar_width as f64).round() as usize;
        let empty_count = bar_width.saturating_sub(filled_count);

        let filled_str = repeat_visible_char(self.filled_char, filled_count);
        let empty_str = repeat_visible_char(self.empty_char, empty_count);

        let mut children: Vec<Element<Msg>> = vec![
            Element::Text(TextElement::new(filled_str).fg(self.filled_color)),
            Element::Text(TextElement::new(empty_str).fg(self.empty_color)),
        ];

        let width = self.width as usize;
        if self.show_percentage && width > 0 {
            let pct_width = width.saturating_sub(bar_width);
            let pct = fit_visible(&format!(" {:3.0}%", value * 100.0), pct_width);
            children.push(Element::Text(TextElement::new(pct)));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }

    pub fn view(&self) -> String {
        let width = self.width as usize;
        if width == 0 {
            return String::new();
        }

        let bar_width = self.bar_width();
        let value = Self::normalize_value(self.value);
        let filled_count = (value * bar_width as f64).round() as usize;
        let empty_count = bar_width.saturating_sub(filled_count);

        let filled = Style::new()
            .fg(self.filled_color)
            .render(&repeat_visible_char(self.filled_char, filled_count));
        let empty = Style::new()
            .fg(self.empty_color)
            .render(&repeat_visible_char(self.empty_char, empty_count));

        let rendered = if self.show_percentage {
            let pct = fit_visible(
                &format!(" {:3.0}%", value * 100.0),
                width.saturating_sub(bar_width),
            );
            format!("{}{}{}", filled, empty, pct)
        } else {
            format!("{}{}", filled, empty)
        };
        fit_visible(&rendered, width)
    }

    fn normalize_value(value: f64) -> f64 {
        if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    fn bar_width(&self) -> usize {
        if self.show_percentage {
            self.width.saturating_sub(5) as usize
        } else {
            self.width as usize
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
    use crate::style::{strip_ansi, visible_len};

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
        let p3 = Progress::new().value(f64::NAN);
        assert_eq!(p3.value, 0.0);
    }

    #[test]
    fn set_value() {
        let mut p = Progress::new();
        p.set_value(0.75);
        assert_eq!(p.value, 0.75);
        p.set_value(f64::INFINITY);
        assert_eq!(p.value, 0.0);
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
    fn zero_width_renders_empty() {
        let p = Progress::new().value(0.5).width(0);

        assert_eq!(p.view(), "");
        let Element::Box(row) = p.element::<()>() else {
            panic!("expected progress row");
        };
        assert_eq!(row.children.len(), 2);
    }

    #[test]
    fn narrow_width_does_not_overflow() {
        let p = Progress::new().value(0.5).width(3);

        assert_eq!(visible_len(&p.view()), 3);
    }

    #[test]
    fn element_percentage_slot_fits_declared_width() {
        let p = Progress::new().value(0.5).width(8);
        let Element::Box(row) = p.element::<()>() else {
            panic!("expected progress row");
        };
        let width = row
            .children
            .iter()
            .map(|child| match child {
                Element::Text(text) => visible_len(&text.content),
                _ => 0,
            })
            .sum::<usize>();

        assert_eq!(width, 8);
    }

    #[test]
    fn custom_wide_glyphs_respect_display_width() {
        let p = Progress::new()
            .value(0.5)
            .width(7)
            .show_percentage(false)
            .filled_char('界')
            .empty_char('好');
        let view = p.view();
        let Element::Box(row) = p.element::<()>() else {
            panic!("expected progress row");
        };
        let element_width = row
            .children
            .iter()
            .map(|child| match child {
                Element::Text(text) => visible_len(&text.content),
                _ => 0,
            })
            .sum::<usize>();

        assert_eq!(visible_len(&strip_ansi(&view)), 7);
        assert_eq!(element_width, 7);
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
