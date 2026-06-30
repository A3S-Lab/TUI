//! Markdown-to-terminal renderer with syntax highlighting.
//!
//! Supports CommonMark via comrak and code highlighting via syntect.

use comrak::nodes::{AstNode, NodeValue};
use comrak::{parse_document, Arena, Options};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SynStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

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
            theme_name: "base16-eighties.dark".to_string(),
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
        let mut options = Options::default();
        options.extension.table = true;
        options.extension.strikethrough = true;
        options.extension.autolink = true;
        options.extension.tasklist = true;
        let root = parse_document(&arena, input, &options);

        let mut output = Vec::new();
        self.render_node(root, &mut output, 0);
        output.join("\n")
    }

    fn render_node<'a>(&self, node: &'a AstNode<'a>, output: &mut Vec<String>, depth: usize) {
        match &node.data.borrow().value {
            NodeValue::Document => {
                for child in node.children() {
                    self.render_node(child, output, depth);
                }
            }
            NodeValue::Heading(heading) => {
                // Clean heading: bold + colour, no literal "#" (Claude-style). A
                // left bar marks h1/h2 for a little hierarchy.
                let text = self.collect_inline(node);
                let lead = if heading.level <= 2 { "▌ " } else { "" };
                let styled = Style::new()
                    .bold()
                    .fg(heading_color(heading.level))
                    .render(&format!("{lead}{text}"));
                output.push(styled);
                output.push(String::new());
            }
            NodeValue::Paragraph => {
                let text = self.collect_inline(node);
                let wrapped = wrap_text(&text, self.width.saturating_sub(depth * 2));
                for line in wrapped {
                    output.push(format!("{}{}", " ".repeat(depth * 2), line));
                }
                // No trailing blank line — keeps chat messages tight (a model that
                // writes one sentence per paragraph would otherwise be double-spaced).
            }
            NodeValue::CodeBlock(cb) => {
                let lang = cb.info.clone();
                let code = cb.literal.clone();
                let highlighted = self.highlight_code(&code, &lang);

                let border_style = Style::new().fg(Color::BrightBlack);
                let top = border_style.render(&format!(
                    "┌─ {} {}",
                    if lang.is_empty() { "code" } else { &lang },
                    "─".repeat(self.width.saturating_sub(lang.len() + 5))
                ));
                let bottom =
                    border_style.render(&format!("└{}", "─".repeat(self.width.saturating_sub(1))));

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
            // Task-list item ("- [x]" / "- [ ]") — render as a plan checklist.
            NodeValue::TaskItem(checked) => {
                let mark = if checked.is_some() {
                    Style::new().fg(Color::Green).bold().render("✔")
                } else {
                    Style::new().fg(Color::BrightBlack).render("□")
                };
                // Same structure as a list item: first paragraph is the checkbox
                // label, the rest renders recursively at depth+1.
                self.render_item_body(node, output, depth, &mark, 1, checked.is_none());
            }
            NodeValue::Item(item) => {
                let bullet = if item.list_type == comrak::nodes::ListType::Ordered {
                    format!("{}.", item.start)
                } else {
                    "•".to_string()
                };
                let bw = bullet.chars().count();
                let bullet_style = Style::new().fg(Color::Rgb(122, 162, 247)).render(&bullet);
                // The item's first paragraph is its bulleted label; every other
                // block child (nested list, code block, extra paragraph) renders
                // through the normal recursive path at depth+1. (Flattening the
                // whole subtree into the label is what rendered nested lists twice.)
                self.render_item_body(node, output, depth, &bullet_style, bw, false);
            }
            NodeValue::BlockQuote => {
                let text = self.collect_inline_from_children(node);
                let bar = Style::new().fg(Color::BrightBlack).render("│");
                let styled_text = Style::new().italic().fg(Color::White).render(&text);
                output.push(format!("{} {}", bar, styled_text));
            }
            NodeValue::ThematicBreak => {
                let rule = Style::new()
                    .fg(Color::BrightBlack)
                    .render(&"─".repeat(self.width));
                output.push(rule);
                output.push(String::new());
            }
            NodeValue::Table(_) => {
                let mut rows: Vec<Vec<String>> = Vec::new();
                for row in node.children() {
                    if matches!(&row.data.borrow().value, NodeValue::TableRow(_)) {
                        rows.push(
                            row.children()
                                .map(|cell| self.collect_inline(cell).trim().to_string())
                                .collect(),
                        );
                    }
                }
                if rows.is_empty() {
                    return;
                }
                let ncols = rows.iter().map(Vec::len).max().unwrap_or(0);
                let mut widths = vec![0usize; ncols];
                for r in &rows {
                    for (i, c) in r.iter().enumerate() {
                        widths[i] = widths[i].max(visible_len(c));
                    }
                }
                let border = Style::new().fg(Color::BrightBlack);
                let rule = |l: &str, m: &str, r: &str| -> String {
                    let segs: Vec<String> = widths.iter().map(|w| "─".repeat(w + 2)).collect();
                    border.render(&format!("{l}{}{r}", segs.join(m)))
                };
                let bar = border.render("│");
                output.push(rule("┌", "┬", "┐"));
                for (ri, cells) in rows.iter().enumerate() {
                    let mut line = bar.clone();
                    for (i, w) in widths.iter().enumerate() {
                        let c = cells.get(i).map(String::as_str).unwrap_or("");
                        let pad = w.saturating_sub(visible_len(c));
                        line.push_str(&format!(" {c}{} ", " ".repeat(pad)));
                        line.push_str(&bar);
                    }
                    output.push(line);
                    if ri == 0 {
                        output.push(rule("├", "┼", "┤"));
                    }
                }
                output.push(rule("└", "┴", "┘"));
            }
            _ => {
                for child in node.children() {
                    self.render_node(child, output, depth);
                }
            }
        }
    }

    /// Push a list item, wrapping long text with a hanging indent so it lines
    /// up under the first character after the bullet. `bullet` is pre-styled;
    /// `bw` is its visible width.
    fn push_list_item(
        &self,
        output: &mut Vec<String>,
        depth: usize,
        bullet: &str,
        bw: usize,
        text: &str,
    ) {
        let prefix_w = depth * 2 + bw + 1;
        let indent = " ".repeat(depth * 2);
        let hang = " ".repeat(prefix_w);
        let avail = self.width.saturating_sub(prefix_w).max(8);
        let wrapped = wrap_text(text, avail);
        for (i, line) in wrapped.iter().enumerate() {
            if i == 0 {
                output.push(format!("{indent}{bullet} {line}"));
            } else {
                output.push(format!("{hang}{line}"));
            }
        }
    }

    /// Render a list-item body: the item's first paragraph becomes the bulleted
    /// label; every other block child (nested list, code block, extra paragraph)
    /// renders through the normal recursive path at `depth + 1`. This is the
    /// single source of truth for item layout — no subtree flattening, so a
    /// nested list renders once (as a sub-list), never also jammed into the label.
    /// `dim` greys the label (unchecked task items).
    fn render_item_body<'a>(
        &self,
        node: &'a AstNode<'a>,
        output: &mut Vec<String>,
        depth: usize,
        bullet: &str,
        bw: usize,
        dim: bool,
    ) {
        let mut labeled = false;
        for child in node.children() {
            if !labeled && matches!(&child.data.borrow().value, NodeValue::Paragraph) {
                let text = self.collect_inline(child);
                let text = if dim {
                    Style::new().fg(Color::BrightBlack).render(&text)
                } else {
                    text
                };
                self.push_list_item(output, depth, bullet, bw, &text);
                labeled = true;
            } else {
                self.render_node(child, output, depth + 1);
            }
        }
        // An item that starts with a sub-list (no leading paragraph) still gets a
        // bullet so the nesting reads correctly.
        if !labeled {
            self.push_list_item(output, depth, bullet, bw, "");
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

// Tokyo Night heading palette (cohesive, low-saturation RGB).
fn heading_color(level: u8) -> Color {
    match level {
        1 => Color::Rgb(122, 162, 247), // blue
        2 => Color::Rgb(187, 154, 247), // purple
        3 => Color::Rgb(125, 207, 255), // cyan
        4 => Color::Rgb(158, 206, 106), // green
        _ => Color::Rgb(192, 202, 245), // fg
    }
}

fn style_to_ansi(style: &SynStyle, text: &str) -> String {
    let fg = style.foreground;
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", fg.r, fg.g, fg.b, text)
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
    fn nested_list_item_not_duplicated() {
        // A nested list must render ONCE (as a sub-list), not also flattened into
        // the parent item's text — regression for "paragraph + its ordered list".
        let md = Markdown::new();
        let output = md.render("1. Parent\n   1. Child A\n   2. Child B");
        let plain = strip_ansi(&output);
        assert_eq!(
            plain.matches("Child A").count(),
            1,
            "nested item rendered twice:\n{plain}"
        );
        assert_eq!(plain.matches("Child B").count(), 1);
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
