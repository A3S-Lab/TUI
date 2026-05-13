//! Markdown-to-terminal renderer with syntax highlighting.
//!
//! Supports CommonMark via comrak and code highlighting via syntect.

use comrak::{parse_document, Arena, Options};
use comrak::nodes::{AstNode, NodeValue};
use syntect::highlighting::{ThemeSet, Style as SynStyle};
use syntect::parsing::SyntaxSet;
use syntect::easy::HighlightLines;

use crate::style::{visible_len, Color, Style};

pub struct Markdown {
    width: usize,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
}

impl Markdown {
    pub fn new() -> Self {
        Self {
            width: 80,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            theme_name: "base16-ocean.dark".to_string(),
        }
    }

    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    pub fn with_theme(mut self, name: &str) -> Self {
        self.theme_name = name.to_string();
        self
    }

    pub fn render(&self, input: &str) -> String {
        let arena = Arena::new();
        let options = Options::default();
        let root = parse_document(&arena, input, &options);

        let mut output = Vec::new();
        self.render_node(root, &mut output, 0);
        output.join("\n")
    }

    fn render_node<'a>(
        &self,
        node: &'a AstNode<'a>,
        output: &mut Vec<String>,
        depth: usize,
    ) {
        match &node.data.borrow().value {
            NodeValue::Document => {
                for child in node.children() {
                    self.render_node(child, output, depth);
                }
            }
            NodeValue::Heading(heading) => {
                let text = self.collect_text(node);
                let prefix = "#".repeat(heading.level as usize);
                let styled = Style::new()
                    .bold()
                    .fg(heading_color(heading.level))
                    .render(&format!("{} {}", prefix, text));
                output.push(styled);
                output.push(String::new());
            }
            NodeValue::Paragraph => {
                let text = self.collect_inline(node);
                let wrapped = wrap_text(&text, self.width.saturating_sub(depth * 2));
                for line in wrapped {
                    output.push(format!("{}{}", " ".repeat(depth * 2), line));
                }
                output.push(String::new());
            }
            NodeValue::CodeBlock(cb) => {
                let lang = cb.info.clone();
                let code = cb.literal.clone();
                let highlighted = self.highlight_code(&code, &lang);

                let border_style = Style::new().fg(Color::BrightBlack);
                let top = border_style.render(&format!("┌─ {} {}", if lang.is_empty() { "code" } else { &lang }, "─".repeat(self.width.saturating_sub(lang.len() + 5))));
                let bottom = border_style.render(&format!("└{}", "─".repeat(self.width.saturating_sub(1))));

                output.push(top);
                for line in highlighted.lines() {
                    output.push(format!("│ {}", line));
                }
                output.push(bottom);
                output.push(String::new());
            }
            NodeValue::List(_) => {
                for child in node.children() {
                    self.render_node(child, output, depth);
                }
            }
            NodeValue::Item(item) => {
                let bullet = if item.list_type == comrak::nodes::ListType::Ordered {
                    format!("{}.", item.start)
                } else {
                    "•".to_string()
                };
                let text = self.collect_inline_from_children(node);
                let indent = " ".repeat(depth * 2);
                let bullet_style = Style::new().fg(Color::Cyan).render(&bullet);
                output.push(format!("{}{} {}", indent, bullet_style, text));

                for child in node.children() {
                    if matches!(&child.data.borrow().value, NodeValue::List(_)) {
                        self.render_node(child, output, depth + 1);
                    }
                }
            }
            NodeValue::BlockQuote => {
                let text = self.collect_inline_from_children(node);
                let bar = Style::new().fg(Color::BrightBlack).render("│");
                let styled_text = Style::new().italic().fg(Color::White).render(&text);
                output.push(format!("{} {}", bar, styled_text));
                output.push(String::new());
            }
            NodeValue::ThematicBreak => {
                let rule = Style::new()
                    .fg(Color::BrightBlack)
                    .render(&"─".repeat(self.width));
                output.push(rule);
                output.push(String::new());
            }
            _ => {
                for child in node.children() {
                    self.render_node(child, output, depth);
                }
            }
        }
    }

    fn collect_text<'a>(&self, node: &'a AstNode<'a>) -> String {
        let mut text = String::new();
        self.collect_text_inner(node, &mut text);
        text
    }

    fn collect_text_inner<'a>(&self, node: &'a AstNode<'a>, buf: &mut String) {
        match &node.data.borrow().value {
            NodeValue::Text(t) => buf.push_str(t),
            NodeValue::Code(c) => buf.push_str(&c.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak => buf.push(' '),
            _ => {
                for child in node.children() {
                    self.collect_text_inner(child, buf);
                }
            }
        }
    }

    fn collect_inline<'a>(&self, node: &'a AstNode<'a>) -> String {
        let mut parts = Vec::new();
        for child in node.children() {
            self.collect_inline_node(child, &mut parts);
        }
        parts.join("")
    }

    fn collect_inline_from_children<'a>(&self, node: &'a AstNode<'a>) -> String {
        let mut parts = Vec::new();
        for child in node.children() {
            if matches!(&child.data.borrow().value, NodeValue::Paragraph) {
                for inner in child.children() {
                    self.collect_inline_node(inner, &mut parts);
                }
            } else {
                self.collect_inline_node(child, &mut parts);
            }
        }
        parts.join("")
    }

    fn collect_inline_node<'a>(&self, node: &'a AstNode<'a>, parts: &mut Vec<String>) {
        match &node.data.borrow().value {
            NodeValue::Text(t) => {
                parts.push(t.clone());
            }
            NodeValue::Code(c) => {
                let code_text = &c.literal;
                let styled = format!("\x1b[48;5;236m\x1b[33m {} \x1b[0m", code_text);
                parts.push(styled);
            }
            NodeValue::Strong => {
                let inner = self.collect_text(node);
                parts.push(format!("\x1b[1m{}\x1b[0m", inner));
            }
            NodeValue::Emph => {
                let inner = self.collect_text(node);
                parts.push(format!("\x1b[3m{}\x1b[0m", inner));
            }
            NodeValue::Strikethrough => {
                let inner = self.collect_text(node);
                parts.push(format!("\x1b[9m{}\x1b[0m", inner));
            }
            NodeValue::Link(link) => {
                let text = self.collect_text(node);
                let url = &link.url;
                parts.push(format!("\x1b[4;34m{}\x1b[0m ({})", text, url));
            }
            NodeValue::SoftBreak | NodeValue::LineBreak => {
                parts.push(" ".to_string());
            }
            _ => {
                for child in node.children() {
                    self.collect_inline_node(child, parts);
                }
            }
        }
    }

    fn highlight_code(&self, code: &str, lang: &str) -> String {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self
            .theme_set
            .themes
            .get(&self.theme_name)
            .unwrap_or_else(|| self.theme_set.themes.values().next().unwrap());

        let mut h = HighlightLines::new(syntax, theme);
        let mut output = Vec::new();

        for line in code.lines() {
            let ranges = h.highlight_line(line, &self.syntax_set).unwrap_or_default();
            let mut styled_line = String::new();
            for (style, text) in ranges {
                styled_line.push_str(&style_to_ansi(&style, text));
            }
            output.push(styled_line);
        }

        output.join("\n")
    }
}

impl Default for Markdown {
    fn default() -> Self {
        Self::new()
    }
}

fn heading_color(level: u8) -> Color {
    match level {
        1 => Color::BrightCyan,
        2 => Color::BrightGreen,
        3 => Color::BrightYellow,
        4 => Color::BrightMagenta,
        _ => Color::White,
    }
}

fn style_to_ansi(style: &SynStyle, text: &str) -> String {
    let fg = style.foreground;
    format!(
        "\x1b[38;2;{};{};{}m{}\x1b[0m",
        fg.r, fg.g, fg.b, text
    )
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_width = visible_len(word);
        let separator_width = if current.is_empty() { 0 } else { 1 };
        if current_width + word_width + separator_width > width && !current.is_empty() {
            lines.push(current);
            current = String::new();
            current_width = 0;
        }
        if !current.is_empty() {
            current.push(' ');
            current_width += 1;
        }
        current.push_str(word);
        current_width += word_width;
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

// Element-based rendering for the new Ink-like architecture
use crate::element::{BoxElement, Element, FlexDirection, TextElement};

impl Markdown {
    pub fn render_element<Msg>(&self, input: &str) -> Element<Msg> {
        let rendered = self.render(input);
        let children: Vec<Element<Msg>> = rendered
            .lines()
            .map(|line| Element::Text(TextElement::new(line)))
            .collect();

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;

    #[test]
    fn render_plain_text() {
        let md = Markdown::new();
        let output = md.render("hello world");
        let plain = strip_ansi(&output);
        assert!(plain.contains("hello world"));
    }

    #[test]
    fn render_heading() {
        let md = Markdown::new();
        let output = md.render("# Title");
        let plain = strip_ansi(&output);
        assert!(plain.contains("Title"));
    }

    #[test]
    fn render_code_block() {
        let md = Markdown::new();
        let output = md.render("```\nlet x = 1;\n```");
        let plain = strip_ansi(&output);
        assert!(plain.contains("let x = 1"));
    }

    #[test]
    fn render_bold() {
        let md = Markdown::new();
        let output = md.render("**bold text**");
        assert!(output.contains("bold text"));
    }

    #[test]
    fn render_list() {
        let md = Markdown::new();
        let output = md.render("- item 1\n- item 2");
        let plain = strip_ansi(&output);
        assert!(plain.contains("item 1"));
        assert!(plain.contains("item 2"));
    }

    #[test]
    fn render_blockquote() {
        let md = Markdown::new();
        let output = md.render("> quoted text");
        let plain = strip_ansi(&output);
        assert!(plain.contains("quoted text"));
    }

    #[test]
    fn render_element_produces_box() {
        let md = Markdown::new();
        let el: Element<()> = md.render_element("# Hello\n\nWorld");
        match el {
            Element::Box(b) => assert!(!b.children.is_empty()),
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn with_width_wraps() {
        let md = Markdown::new().with_width(10);
        let output = md.render("this is a long sentence that should wrap");
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() > 1);
    }

    #[test]
    fn wrap_text_basic() {
        let lines = wrap_text("hello world foo bar", 10);
        assert!(lines.len() >= 2);
        for line in &lines {
            assert!(crate::style::visible_len(line) <= 10);
        }
    }
}
