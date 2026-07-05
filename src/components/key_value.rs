use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, Color, Style};

const MAX_KEY_VALUE_INDENT: usize = u16::MAX as usize;
const MAX_KEY_VALUE_KEY_WIDTH: usize = u16::MAX as usize;

/// A key-value pair display component.
///
/// Renders a list of labeled values in a consistent format.
pub struct KeyValue {
    pairs: Vec<(String, String)>,
    key_color: Color,
    value_color: Color,
    separator: String,
    indent: usize,
    key_width: Option<usize>,
}

impl KeyValue {
    pub fn new() -> Self {
        Self {
            pairs: Vec::new(),
            key_color: Color::BrightBlack,
            value_color: Color::White,
            separator: ": ".to_string(),
            indent: 0,
            key_width: None,
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

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent.min(MAX_KEY_VALUE_INDENT);
        self
    }

    pub fn key_width(mut self, width: usize) -> Self {
        self.key_width = Some(width.min(MAX_KEY_VALUE_KEY_WIDTH));
        self
    }

    pub fn lines(&self, width: u16) -> Vec<String> {
        let width = width as usize;
        if width == 0 {
            return Vec::new();
        }

        self.pairs
            .iter()
            .map(|(key, value)| self.render_line(key, value, width))
            .collect()
    }

    pub fn view(&self, width: u16) -> String {
        self.lines(width).join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let children: Vec<Element<Msg>> = self
            .pairs
            .iter()
            .map(|(key, value)| {
                let key = self.key_label(key, usize::MAX);
                Element::Box(
                    BoxElement::new()
                        .direction(FlexDirection::Row)
                        .child(Element::Text(
                            TextElement::new(key.as_str()).fg(self.key_color),
                        ))
                        .child(Element::Text(TextElement::new(&self.separator)))
                        .child(Element::Text(
                            TextElement::new(value.as_str()).fg(self.value_color),
                        )),
                )
            })
            .collect();

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_line(&self, key: &str, value: &str, width: usize) -> String {
        let indent_width = self.indent.min(width).min(MAX_KEY_VALUE_INDENT);
        let available = width.saturating_sub(indent_width);
        let key = self.key_label(key, available);
        let raw = format!(
            "{}{}{}{}",
            " ".repeat(indent_width),
            Style::new().fg(self.key_color).render(&key),
            self.separator,
            Style::new().fg(self.value_color).render(value)
        );
        fit_visible(&raw, width)
    }

    fn key_label(&self, key: &str, available: usize) -> String {
        match self.key_width {
            Some(width) => fit_visible(key, width.min(available).min(MAX_KEY_VALUE_KEY_WIDTH)),
            None => key.to_string(),
        }
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
    use crate::style::{strip_ansi, visible_len};

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

    #[test]
    fn view_renders_styled_width_bounded_rows() {
        let rendered = KeyValue::new()
            .pair("Name", "a3s-tui")
            .pair("Version", "0.1.0")
            .view(18);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("Name: a3s-tui"), "{plain}");
        assert!(
            rendered.contains("\x1b["),
            "key/value rows should carry styling"
        );
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 18, "{line:?}");
        }
    }

    #[test]
    fn key_width_aligns_values_in_line_rendering() {
        let rendered = KeyValue::new()
            .key_width(8)
            .separator(" ")
            .pair("pid", "42")
            .pair("workspace", "a3s")
            .view(24);
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert!(rows[0].contains("pid      42"), "{plain}");
        assert!(rows[1].starts_with("workspa"), "{plain}");
        assert!(rows[1].contains("a3s"), "{plain}");
    }

    #[test]
    fn zero_width_renders_no_lines() {
        let kv = KeyValue::new().pair("key", "value");

        assert!(kv.lines(0).is_empty());
        assert_eq!(kv.view(0), "");
    }

    #[test]
    fn oversized_spacing_is_clamped_to_render_width() {
        let kv = KeyValue::new()
            .indent(usize::MAX)
            .key_width(usize::MAX)
            .pair("workspace", "a3s");
        let rendered = kv.view(8);

        assert_eq!(kv.indent, MAX_KEY_VALUE_INDENT);
        assert_eq!(kv.key_width, Some(MAX_KEY_VALUE_KEY_WIDTH));
        assert!(rendered.lines().all(|line| visible_len(line) == 8));
    }
}
