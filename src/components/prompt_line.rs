use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, visible_len, Color, Style};

/// Prompt-prefixed input text with aligned continuation rows.
///
/// This extracts the common CLI input pattern: a left margin, a styled prompt
/// prefix on the first row, and continuation rows indented to the same content
/// column using display-width accounting.
#[derive(Debug, Clone)]
pub struct PromptLine {
    prompt: String,
    text: String,
    margin: usize,
    width: Option<usize>,
    continuation_indent: Option<usize>,
    prompt_style: Option<Style>,
    text_style: Option<Style>,
}

impl PromptLine {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            text: String::new(),
            margin: 0,
            width: None,
            continuation_indent: None,
            prompt_style: None,
            text_style: None,
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin;
        self
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the total display-column indent for continuation rows.
    pub fn continuation_indent(mut self, indent: usize) -> Self {
        self.continuation_indent = Some(indent);
        self
    }

    pub fn prompt_style(mut self, style: Style) -> Self {
        self.prompt_style = Some(style);
        self
    }

    pub fn prompt_color(mut self, color: Color) -> Self {
        self.prompt_style = Some(Style::new().fg(color).bold());
        self
    }

    pub fn text_style(mut self, style: Style) -> Self {
        self.text_style = Some(style);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_style = Some(Style::new().fg(color));
        self
    }

    pub fn prompt_value(&self) -> &str {
        &self.prompt
    }

    pub fn text_value(&self) -> &str {
        &self.text
    }

    pub fn view(&self) -> String {
        self.render_lines()
            .into_iter()
            .map(|line| {
                self.width
                    .map(|width| fit_visible(&line, width))
                    .unwrap_or(line)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        Element::Box(
            BoxElement::new().direction(FlexDirection::Column).children(
                self.text_lines()
                    .into_iter()
                    .enumerate()
                    .map(|(index, line)| self.element_line(index, line))
                    .collect(),
            ),
        )
    }

    fn render_lines(&self) -> Vec<String> {
        self.text_lines()
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                if index == 0 {
                    let prompt = match &self.prompt_style {
                        Some(style) => style.render(&self.prompt),
                        None => self.prompt.clone(),
                    };
                    let text = self.render_text(&line);
                    format!("{}{prompt}{text}", " ".repeat(self.margin))
                } else {
                    format!(
                        "{}{}",
                        " ".repeat(self.continuation_indent_width()),
                        self.render_text(&line)
                    )
                }
            })
            .collect()
    }

    fn element_line<Msg>(&self, index: usize, line: String) -> Element<Msg> {
        if index == 0 {
            let mut prompt = TextElement::new(self.prompt.as_str());
            if let Some(style) = &self.prompt_style {
                prompt = apply_text_style(prompt, style);
            }

            let mut text = TextElement::new(line);
            if let Some(style) = &self.text_style {
                text = apply_text_style(text, style);
            }

            Element::Box(
                BoxElement::new()
                    .direction(FlexDirection::Row)
                    .child(Element::Text(TextElement::new(" ".repeat(self.margin))))
                    .child(Element::Text(prompt))
                    .child(Element::Text(text)),
            )
        } else {
            let mut text = TextElement::new(line);
            if let Some(style) = &self.text_style {
                text = apply_text_style(text, style);
            }

            Element::Box(
                BoxElement::new()
                    .direction(FlexDirection::Row)
                    .child(Element::Text(TextElement::new(
                        " ".repeat(self.continuation_indent_width()),
                    )))
                    .child(Element::Text(text)),
            )
        }
    }

    fn text_lines(&self) -> Vec<String> {
        let lines = self
            .text
            .split('\n')
            .map(str::to_string)
            .collect::<Vec<_>>();
        if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        }
    }

    fn render_text(&self, text: &str) -> String {
        match &self.text_style {
            Some(style) => style.render(text),
            None => text.to_string(),
        }
    }

    fn continuation_indent_width(&self) -> usize {
        self.continuation_indent
            .unwrap_or_else(|| self.margin + visible_len(&self.prompt))
    }
}

impl Default for PromptLine {
    fn default() -> Self {
        Self::new("")
    }
}

fn apply_text_style(mut text: TextElement, style: &Style) -> TextElement {
    if let Some(fg) = style.foreground() {
        text = text.fg(fg);
    }
    if let Some(bg) = style.background() {
        text = text.bg(bg);
    }
    if style.is_bold() {
        text = text.bold();
    }
    if style.is_italic() {
        text = text.italic();
    }
    if style.is_underline() {
        text = text.underline();
    }
    if style.is_dim() {
        text = text.dim();
    }
    if style.is_strikethrough() {
        text = text.strikethrough();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn renders_prompt_and_aligned_continuation_rows() {
        let rendered = PromptLine::new("❯ ")
            .text("cargo test\n--all-targets")
            .margin(2)
            .width(24)
            .prompt_style(Style::new().fg(Color::Cyan).bold())
            .view();
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert!(rows[0].starts_with("  ❯ cargo test"));
        assert!(rows[1].starts_with("    --all-targets"));
        assert!(rendered.lines().all(|line| visible_len(line) == 24));
        assert!(rendered.contains("\x1b[1;36m❯ \x1b[0m"));
    }

    #[test]
    fn styles_text_separately_from_prompt() {
        let rendered = PromptLine::new("? ")
            .text("research mode")
            .margin(1)
            .prompt_color(Color::Cyan)
            .text_color(Color::BrightWhite)
            .view();

        assert_eq!(strip_ansi(&rendered), " ? research mode");
        assert!(rendered.contains("\x1b[1;36m? \x1b[0m"));
        assert!(rendered.contains("\x1b[97mresearch mode\x1b[0m"));
    }

    #[test]
    fn custom_continuation_indent_overrides_prompt_width() {
        let rendered = PromptLine::new("> ")
            .text("one\ntwo")
            .margin(2)
            .continuation_indent(8)
            .view();
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert_eq!(rows[0], "  > one");
        assert_eq!(rows[1], "        two");
    }

    #[test]
    fn wide_prompt_uses_display_width_for_continuation_indent() {
        let rendered = PromptLine::new("💬 ").text("hello\nworld").margin(1).view();
        let plain = strip_ansi(&rendered);
        let rows = plain.lines().collect::<Vec<_>>();

        assert!(rows[0].starts_with(" 💬 hello"));
        assert_eq!(rows[1], "    world");
    }

    #[test]
    fn element_produces_structured_rows() {
        let element: Element<()> = PromptLine::new("❯ ")
            .text("alpha\nbeta")
            .margin(2)
            .prompt_color(Color::Cyan)
            .text_color(Color::BrightWhite)
            .element();

        match element {
            Element::Box(column) => {
                assert_eq!(column.children.len(), 2);
                match &column.children[0] {
                    Element::Box(row) => {
                        assert_eq!(row.children.len(), 3);
                        match &row.children[1] {
                            Element::Text(prompt) => {
                                assert_eq!(prompt.content, "❯ ");
                                assert_eq!(prompt.style.fg, Some(Color::Cyan));
                                assert!(prompt.style.bold);
                            }
                            _ => panic!("expected prompt text"),
                        }
                    }
                    _ => panic!("expected first row"),
                }
            }
            _ => panic!("expected column element"),
        }
    }
}
