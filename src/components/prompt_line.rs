use crate::element::{BoxElement, Dimension, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_PROMPT_LINE_CONTINUATION_INDENT: usize = u16::MAX as usize;
const MAX_PROMPT_LINE_MARGIN: usize = u16::MAX as usize;
const MAX_PROMPT_LINE_WIDTH: usize = u16::MAX as usize;

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
    background_color: Option<Color>,
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
            background_color: None,
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin.min(MAX_PROMPT_LINE_MARGIN);
        self
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width.min(MAX_PROMPT_LINE_WIDTH));
        self
    }

    /// Set the total display-column indent for continuation rows.
    pub fn continuation_indent(mut self, indent: usize) -> Self {
        self.continuation_indent = Some(indent.min(MAX_PROMPT_LINE_CONTINUATION_INDENT));
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

    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.prompt_style = Some(theme.foreground_style(ThemeRole::Primary).bold());
        self.text_style = Some(theme.foreground_style(ThemeRole::Foreground));
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
            .map(|line| self.fit_rendered_line(line))
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
        let margin = self.margin_for_render();
        let continuation_indent = self.continuation_indent_for_render();
        self.text_lines()
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                if index == 0 {
                    let prompt = self.render_piece(&self.prompt, self.prompt_style.as_ref());
                    let text = self.render_text(&line);
                    let margin = self.render_piece(&" ".repeat(margin), None);
                    format!("{margin}{prompt}{text}")
                } else {
                    let indent = self.render_piece(&" ".repeat(continuation_indent), None);
                    format!("{indent}{}", self.render_text(&line))
                }
            })
            .collect()
    }

    fn element_line<Msg>(&self, index: usize, line: String) -> Element<Msg> {
        let mut row = BoxElement::new().direction(FlexDirection::Row);
        if let Some(background) = self.background_color {
            row = row.bg(background);
            if let Some(width) = self.width {
                row = row.width(Dimension::Points(width as f32));
            }
        }

        if index == 0 {
            let mut prompt = TextElement::new(self.prompt.as_str());
            if let Some(style) = &self.prompt_style {
                prompt = apply_text_style(prompt, style);
            }
            prompt = self.apply_background(prompt);

            let mut text = TextElement::new(line);
            if let Some(style) = &self.text_style {
                text = apply_text_style(text, style);
            }
            text = self.apply_background(text);

            Element::Box(
                row.child(Element::Text(self.apply_background(TextElement::new(
                    " ".repeat(self.margin_for_render()),
                ))))
                .child(Element::Text(prompt))
                .child(Element::Text(text)),
            )
        } else {
            let mut text = TextElement::new(line);
            if let Some(style) = &self.text_style {
                text = apply_text_style(text, style);
            }
            text = self.apply_background(text);

            Element::Box(
                row.child(Element::Text(self.apply_background(TextElement::new(
                    " ".repeat(self.continuation_indent_for_render()),
                ))))
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
        self.render_piece(text, self.text_style.as_ref())
    }

    fn render_piece(&self, text: &str, style: Option<&Style>) -> String {
        let mut style = style.cloned().unwrap_or_default();
        if let Some(background) = self.background_color {
            style = style.bg(background);
        }
        style.render(text)
    }

    fn fit_rendered_line(&self, line: String) -> String {
        let Some(width) = self.width else {
            return line;
        };
        if self.background_color.is_none() {
            return fit_visible(&line, width);
        }

        let line = truncate_visible(&line, width);
        let padding = width.saturating_sub(visible_len(&line));
        let padding = self.render_piece(&" ".repeat(padding), None);
        format!("{line}{padding}")
    }

    fn apply_background(&self, mut text: TextElement) -> TextElement {
        if let Some(background) = self.background_color {
            text = text.bg(background);
        }
        text
    }

    fn continuation_indent_width(&self) -> usize {
        self.continuation_indent
            .unwrap_or_else(|| self.margin.saturating_add(visible_len(&self.prompt)))
            .min(MAX_PROMPT_LINE_CONTINUATION_INDENT)
    }

    fn margin_for_render(&self) -> usize {
        self.width
            .map(|width| self.margin.min(width))
            .unwrap_or(self.margin)
            .min(MAX_PROMPT_LINE_MARGIN)
    }

    fn continuation_indent_for_render(&self) -> usize {
        let indent = self.continuation_indent_width();
        self.width
            .map(|width| indent.min(width))
            .unwrap_or(indent)
            .min(MAX_PROMPT_LINE_CONTINUATION_INDENT)
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
    if style.is_reverse() {
        text = text.reverse();
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
    fn background_fills_full_width_while_preserving_prompt_and_text_styles() {
        let background = Color::Rgb(31, 31, 31);
        let line = PromptLine::new("❯ ")
            .text("cargo test\n--all")
            .margin(2)
            .width(16)
            .prompt_color(Color::Cyan)
            .text_color(Color::BrightWhite)
            .background_color(background);
        let rendered = line.view();
        let rows = rendered.lines().collect::<Vec<_>>();

        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| visible_len(row) == 16));
        assert_eq!(strip_ansi(rows[0]), "  ❯ cargo test  ");
        assert_eq!(strip_ansi(rows[1]), "    --all       ");
        assert!(rendered.contains("\x1b[1;36;48;2;31;31;31m❯ \x1b[0m"));
        assert!(rendered.contains("\x1b[97;48;2;31;31;31mcargo test\x1b[0m"));
        assert!(rendered.contains("\x1b[48;2;31;31;31m  \x1b[0m"));

        let Element::Box(column) = line.element::<()>() else {
            panic!("expected column element");
        };
        for child in &column.children {
            let Element::Box(row) = child else {
                panic!("expected row element");
            };
            assert_eq!(row.style.bg, Some(background));
            assert_eq!(row.style.width, Dimension::Points(16.0));
        }
        let Element::Box(first_row) = &column.children[0] else {
            panic!("expected first row");
        };
        let Element::Text(prompt) = &first_row.children[1] else {
            panic!("expected prompt text");
        };
        let Element::Text(text) = &first_row.children[2] else {
            panic!("expected input text");
        };
        assert_eq!(prompt.style.fg, Some(Color::Cyan));
        assert_eq!(prompt.style.bg, Some(background));
        assert_eq!(text.style.fg, Some(Color::BrightWhite));
        assert_eq!(text.style.bg, Some(background));
    }

    #[test]
    fn with_theme_applies_semantic_styles() {
        let theme = Theme::tokyo_night();
        let line = PromptLine::new("❯ ").with_theme(&theme);
        let prompt_style = line.prompt_style.as_ref().expect("prompt style");
        let text_style = line.text_style.as_ref().expect("text style");

        assert_eq!(
            prompt_style.foreground(),
            Some(theme.color(ThemeRole::Primary))
        );
        assert!(prompt_style.is_bold());
        assert_eq!(
            text_style.foreground(),
            Some(theme.color(ThemeRole::Foreground))
        );
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
    fn oversized_spacing_is_clamped_to_render_width() {
        let line = PromptLine::new("> ")
            .text("hello\nworld")
            .margin(usize::MAX)
            .continuation_indent(usize::MAX)
            .width(8);
        let rendered = line.view();
        let rows = rendered.lines().collect::<Vec<_>>();

        assert_eq!(line.margin, MAX_PROMPT_LINE_MARGIN);
        assert_eq!(
            line.continuation_indent,
            Some(MAX_PROMPT_LINE_CONTINUATION_INDENT)
        );
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| visible_len(row) == 8));

        let Element::Box(column) = line.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Box(first_row) = &column.children[0] else {
            panic!("expected first row");
        };
        let Element::Text(margin) = &first_row.children[0] else {
            panic!("expected margin text");
        };
        assert_eq!(margin.content.len(), 8);

        let Element::Box(second_row) = &column.children[1] else {
            panic!("expected second row");
        };
        let Element::Text(indent) = &second_row.children[0] else {
            panic!("expected continuation indent text");
        };
        assert_eq!(indent.content.len(), 8);
    }

    #[test]
    fn oversized_width_is_clamped() {
        let line = PromptLine::new("> ").text("hello").width(usize::MAX);
        let rendered = line.view();

        assert_eq!(line.width, Some(MAX_PROMPT_LINE_WIDTH));
        assert_eq!(visible_len(&rendered), MAX_PROMPT_LINE_WIDTH);
    }

    #[test]
    fn element_produces_structured_rows() {
        let element: Element<()> = PromptLine::new("❯ ")
            .text("alpha\nbeta")
            .margin(2)
            .prompt_style(Style::new().fg(Color::Cyan).bold().reverse())
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
                                assert!(prompt.style.reverse);
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
