//! Markdown-to-terminal renderer.
//!
//! Supports CommonMark via comrak. Code highlighting is enabled by the
//! `syntax-highlighting` crate feature.

use comrak::nodes::{AstNode, NodeValue};
use comrak::{parse_document, Arena, Options};
#[cfg(feature = "syntax-highlighting")]
use syntect::easy::HighlightLines;
#[cfg(feature = "syntax-highlighting")]
use syntect::highlighting::{Style as SynStyle, ThemeSet};
#[cfg(feature = "syntax-highlighting")]
use syntect::parsing::SyntaxSet;

use crate::style::{
    fit_visible, next_display_cell_boundary, split_lines_preserving_trailing_blank,
    truncate_visible, visible_len, Color, Style,
};

const MAX_MARKDOWN_WIDTH: usize = u16::MAX as usize;

pub struct Markdown {
    width: usize,
    #[cfg(feature = "syntax-highlighting")]
    syntax_set: SyntaxSet,
    #[cfg(feature = "syntax-highlighting")]
    theme_set: ThemeSet,
    #[cfg(feature = "syntax-highlighting")]
    theme_name: String,
}

impl Markdown {
    pub fn new() -> Self {
        Self {
            width: 80,
            #[cfg(feature = "syntax-highlighting")]
            syntax_set: SyntaxSet::load_defaults_newlines(),
            #[cfg(feature = "syntax-highlighting")]
            theme_set: ThemeSet::load_defaults(),
            #[cfg(feature = "syntax-highlighting")]
            theme_name: "base16-eighties.dark".to_string(),
        }
    }

    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width.min(MAX_MARKDOWN_WIDTH);
        self
    }

    pub fn with_theme(mut self, name: &str) -> Self {
        #[cfg(feature = "syntax-highlighting")]
        {
            self.theme_name = name.to_string();
        }
        #[cfg(not(feature = "syntax-highlighting"))]
        {
            let _ = name;
        }
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
                let label = format!("{lead}{text}");
                let label = if self.width == 0 {
                    label
                } else {
                    truncate_visible(&label, self.width)
                };
                let styled = Style::new()
                    .bold()
                    .fg(heading_color(heading.level))
                    .render(&label);
                output.push(styled);
                output.push(String::new());
            }
            NodeValue::Paragraph => {
                let text = self.collect_inline(node);
                let wrapped = wrap_text(&text, self.width.saturating_sub(depth * 2));
                for line in wrapped {
                    output.push(fit_markdown_line(
                        format!("{}{}", " ".repeat(depth * 2), line),
                        self.width,
                    ));
                }
                // No trailing blank line — keeps chat messages tight (a model that
                // writes one sentence per paragraph would otherwise be double-spaced).
            }
            NodeValue::CodeBlock(cb) => {
                let lang = cb.info.clone();
                let code = cb.literal.clone();
                let highlighted = self.highlight_code(&code, &lang);

                let border_style = Style::new().fg(Color::BrightBlack);
                let label = if lang.is_empty() {
                    "code"
                } else {
                    lang.as_str()
                };
                let top = border_style.render(&code_block_header(label, self.width));
                let bottom = border_style.render(&code_block_footer(self.width));

                output.push(top);
                for line in split_lines_preserving_trailing_blank(&highlighted) {
                    output.push(code_block_body_line(line, self.width));
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
                let bw = visible_len(&bullet);
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
                if self.width == 0 {
                    let styled_text = Style::new().italic().fg(Color::White).render(&text);
                    output.push(format!("{} {}", bar, styled_text));
                } else if self.width == 1 {
                    output.push(bar);
                } else {
                    for line in wrap_text(&text, self.width - 2) {
                        let styled_text = Style::new().italic().fg(Color::White).render(&line);
                        output.push(format!("{} {}", bar, styled_text));
                    }
                }
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
                let widths = constrain_table_widths(&widths, self.width);
                let border = Style::new().fg(Color::BrightBlack);
                let rule = |l: &str, m: &str, r: &str| -> String {
                    let segs: Vec<String> = widths.iter().map(|w| "─".repeat(w + 2)).collect();
                    fit_markdown_line(
                        border.render(&format!("{l}{}{r}", segs.join(m))),
                        self.width,
                    )
                };
                let bar = border.render("│");
                output.push(rule("┌", "┬", "┐"));
                for (ri, cells) in rows.iter().enumerate() {
                    let mut line = bar.clone();
                    for (i, w) in widths.iter().enumerate() {
                        let c = cells.get(i).map(String::as_str).unwrap_or("");
                        line.push_str(&format!(" {} ", fit_visible(c, *w)));
                        line.push_str(&bar);
                    }
                    output.push(fit_markdown_line(line, self.width));
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
                output.push(fit_markdown_line(
                    format!("{indent}{bullet} {line}"),
                    self.width,
                ));
            } else {
                output.push(fit_markdown_line(format!("{hang}{line}"), self.width));
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

    #[cfg(feature = "syntax-highlighting")]
    fn highlight_code(&self, code: &str, lang: &str) -> String {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let Some(theme) = self
            .theme_set
            .themes
            .get(&self.theme_name)
            .or_else(|| self.theme_set.themes.values().next())
        else {
            return code.to_string();
        };

        let mut h = HighlightLines::new(syntax, theme);
        let mut output = Vec::new();

        for line in split_lines_preserving_trailing_blank(code) {
            let ranges = h.highlight_line(line, &self.syntax_set).unwrap_or_default();
            let mut styled_line = String::new();
            for (style, text) in ranges {
                styled_line.push_str(&style_to_ansi(&style, text));
            }
            output.push(styled_line);
        }

        output.join("\n")
    }

    #[cfg(not(feature = "syntax-highlighting"))]
    fn highlight_code(&self, code: &str, _lang: &str) -> String {
        code.to_string()
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

#[cfg(feature = "syntax-highlighting")]
fn style_to_ansi(style: &SynStyle, text: &str) -> String {
    let fg = style.foreground;
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", fg.r, fg.g, fg.b, text)
}

fn code_block_header(label: &str, width: usize) -> String {
    match width {
        0 => String::new(),
        1 => "┌".to_string(),
        2 => "┌─".to_string(),
        3 => "┌─ ".to_string(),
        _ => {
            let label = truncate_visible(label, width - 4);
            let prefix = format!("┌─ {label} ");
            format!(
                "{prefix}{}",
                "─".repeat(width.saturating_sub(visible_len(&prefix)))
            )
        }
    }
}

fn code_block_footer(width: usize) -> String {
    match width {
        0 => String::new(),
        _ => format!("└{}", "─".repeat(width.saturating_sub(1))),
    }
}

fn code_block_body_line(line: &str, width: usize) -> String {
    match width {
        0 => String::new(),
        1 => "│".to_string(),
        _ => format!("│ {}", truncate_visible(line, width - 2)),
    }
}

fn constrain_table_widths(widths: &[usize], max_width: usize) -> Vec<usize> {
    if max_width == 0 || widths.is_empty() {
        return widths.to_vec();
    }

    let overhead = widths.len().saturating_mul(3).saturating_add(1);
    if overhead >= max_width {
        return vec![0; widths.len()];
    }

    let mut budget = max_width - overhead;
    if widths.iter().sum::<usize>() <= budget {
        return widths.to_vec();
    }

    let mut constrained = vec![0; widths.len()];
    for (i, width) in widths.iter().enumerate() {
        if budget == 0 {
            break;
        }
        if *width > 0 {
            constrained[i] = 1;
            budget -= 1;
        }
    }

    while budget > 0 {
        let Some((index, _)) = widths
            .iter()
            .enumerate()
            .filter(|(index, width)| constrained[*index] < **width)
            .max_by_key(|(index, width)| width.saturating_sub(constrained[*index]))
        else {
            break;
        };
        constrained[index] += 1;
        budget -= 1;
    }

    constrained
}

fn fit_markdown_line(line: String, width: usize) -> String {
    if width == 0 {
        line
    } else {
        truncate_visible(&line, width)
    }
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
        if word_width > width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }

            for part in split_visible_word(word, width) {
                let part_width = visible_len(&part);
                if part_width >= width {
                    lines.push(part);
                } else {
                    current = part;
                    current_width = part_width;
                }
            }
            continue;
        }

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

fn split_visible_word(word: &str, width: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut index = 0usize;

    while index < word.len() {
        let ch = word[index..].chars().next().unwrap_or_default();
        if ch == '\x1b' && word[index + ch.len_utf8()..].starts_with('[') {
            current.push(ch);
            let escape_start = index + ch.len_utf8();
            for (offset, next) in word[escape_start..].char_indices() {
                current.push(next);
                index = escape_start + offset + next.len_utf8();
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }

        let Some((end, ch_width)) = next_display_cell_boundary(word, index) else {
            break;
        };
        let cell = &word[index..end];
        index = end;

        if ch_width == 0 {
            current.push_str(cell);
            continue;
        }

        if ch_width > width {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
                current_width = 0;
            }
            parts.push(truncate_visible(cell, width));
            continue;
        }
        if current_width > 0 && current_width + ch_width > width {
            parts.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push_str(cell);
        current_width += ch_width;
        if current_width >= width {
            parts.push(std::mem::take(&mut current));
            current_width = 0;
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

// Element-based rendering for the new Ink-like architecture
use crate::element::{BoxElement, Element, FlexDirection, TextElement, TextStyle};

struct StyledSegment {
    text: String,
    style: TextStyle,
}

impl Markdown {
    pub fn render_element<Msg>(&self, input: &str) -> Element<Msg> {
        let rendered = self.render(input);
        let children: Vec<Element<Msg>> = split_rendered_lines(&rendered)
            .into_iter()
            .map(ansi_line_element)
            .collect();

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }
}

pub(crate) fn split_rendered_lines(rendered: &str) -> Vec<&str> {
    if rendered.is_empty() {
        Vec::new()
    } else {
        split_lines_preserving_trailing_blank(rendered)
    }
}

fn ansi_line_element<Msg>(line: &str) -> Element<Msg> {
    let segments = ansi_segments(line);
    if segments.len() == 1 {
        let Some(segment) = segments.into_iter().next() else {
            return Element::Text(TextElement::new(""));
        };
        return Element::Text(segment_text(segment));
    }

    Element::Box(
        BoxElement::new().direction(FlexDirection::Row).children(
            segments
                .into_iter()
                .map(|segment| Element::Text(segment_text(segment)))
                .collect(),
        ),
    )
}

fn segment_text(segment: StyledSegment) -> TextElement {
    let mut text = TextElement::new(segment.text);
    text.style = segment.style;
    text
}

fn ansi_segments(line: &str) -> Vec<StyledSegment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut style = TextStyle::default();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            let mut sequence = String::new();
            let mut final_byte = None;
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    final_byte = Some(next);
                    break;
                }
                sequence.push(next);
            }

            if final_byte == Some('m') {
                push_segment(&mut segments, &mut current, &style);
                apply_sgr_sequence(&mut style, &sequence);
            }
            continue;
        }

        current.push(ch);
    }

    push_segment(&mut segments, &mut current, &style);
    if segments.is_empty() {
        segments.push(StyledSegment {
            text: String::new(),
            style: TextStyle::default(),
        });
    }
    segments
}

fn push_segment(segments: &mut Vec<StyledSegment>, current: &mut String, style: &TextStyle) {
    if current.is_empty() {
        return;
    }

    segments.push(StyledSegment {
        text: std::mem::take(current),
        style: style.clone(),
    });
}

fn apply_sgr_sequence(style: &mut TextStyle, sequence: &str) {
    if sequence.is_empty() {
        *style = TextStyle::default();
        return;
    }

    let codes: Vec<u16> = sequence
        .split(';')
        .filter_map(|part| {
            if part.is_empty() {
                Some(0)
            } else {
                part.parse().ok()
            }
        })
        .collect();
    if codes.is_empty() {
        *style = TextStyle::default();
        return;
    }

    let mut index = 0usize;
    while index < codes.len() {
        match codes[index] {
            0 => *style = TextStyle::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            4 => style.underline = true,
            7 => style.reverse = true,
            9 => style.strikethrough = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            23 => style.italic = false,
            24 => style.underline = false,
            27 => style.reverse = false,
            29 => style.strikethrough = false,
            30..=37 | 90..=97 => style.fg = basic_sgr_color(codes[index]),
            39 => style.fg = None,
            40..=47 | 100..=107 => style.bg = basic_sgr_color(codes[index] - 10),
            49 => style.bg = None,
            38 | 48 => {
                let is_fg = codes[index] == 38;
                if let Some((color, consumed)) = extended_sgr_color(&codes[index + 1..]) {
                    if is_fg {
                        style.fg = Some(color);
                    } else {
                        style.bg = Some(color);
                    }
                    index += consumed;
                }
            }
            _ => {}
        }
        index += 1;
    }
}

fn basic_sgr_color(code: u16) -> Option<Color> {
    match code {
        30 => Some(Color::Black),
        31 => Some(Color::Red),
        32 => Some(Color::Green),
        33 => Some(Color::Yellow),
        34 => Some(Color::Blue),
        35 => Some(Color::Magenta),
        36 => Some(Color::Cyan),
        37 => Some(Color::White),
        90 => Some(Color::BrightBlack),
        91 => Some(Color::BrightRed),
        92 => Some(Color::BrightGreen),
        93 => Some(Color::BrightYellow),
        94 => Some(Color::BrightBlue),
        95 => Some(Color::BrightMagenta),
        96 => Some(Color::BrightCyan),
        97 => Some(Color::BrightWhite),
        _ => None,
    }
}

fn extended_sgr_color(codes: &[u16]) -> Option<(Color, usize)> {
    match codes {
        [5, value, ..] => Some((Color::Ansi256((*value).min(u8::MAX as u16) as u8), 2)),
        [2, r, g, b, ..] => Some((
            Color::Rgb(
                (*r).min(u8::MAX as u16) as u8,
                (*g).min(u8::MAX as u16) as u8,
                (*b).min(u8::MAX as u16) as u8,
            ),
            4,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

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
    fn heading_respects_configured_width() {
        let md = Markdown::new().with_width(8);
        let output = md.render("# abcdefghijklmnopqrstuvwxyz");
        let heading = output.lines().next().unwrap();

        assert!(visible_len(heading) <= 8, "{heading:?}");
    }

    #[test]
    fn render_code_block() {
        let md = Markdown::new();
        let output = md.render("```\nlet x = 1;\n```");
        let plain = strip_ansi(&output);
        assert!(plain.contains("let x = 1"));
    }

    #[test]
    fn render_code_block_preserves_trailing_blank_body_line() {
        let md = Markdown::new();
        let output = md.render("```rust\nlet x = 1;\n```");
        let plain = strip_ansi(&output);
        let rows = plain.lines().collect::<Vec<_>>();

        assert!(rows[0].starts_with("┌─ rust"));
        assert_eq!(rows[1], "│ let x = 1;");
        assert_eq!(rows[2], "│ ");
        assert!(rows[3].starts_with("└"));
    }

    #[test]
    fn code_block_header_fits_width_without_language() {
        let md = Markdown::new().with_width(12);
        let output = md.render("```\ntext\n```");
        let top = output.lines().next().unwrap();

        assert_eq!(visible_len(top), 12);
    }

    #[test]
    fn code_block_header_uses_language_display_width() {
        let md = Markdown::new().with_width(12);
        let output = md.render("```文件\ntext\n```");
        let top = output.lines().next().unwrap();

        assert_eq!(visible_len(top), 12);
    }

    #[test]
    fn code_block_footer_respects_zero_width() {
        let md = Markdown::new().with_width(0);
        let output = md.render("```\ntext\n```");
        let footer = output.lines().last().unwrap();

        assert_eq!(visible_len(footer), 0);
    }

    #[test]
    fn code_block_body_lines_respect_width() {
        let md = Markdown::new().with_width(8);
        let output = md.render("```\nabcdefghijklmnopqrstuvwxyz\n```");

        for line in output.lines() {
            assert!(visible_len(line) <= 8, "{line:?}");
        }
    }

    #[cfg(feature = "syntax-highlighting")]
    #[test]
    fn render_code_block_without_themes_falls_back_to_plain_text() {
        let md = Markdown {
            width: 80,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::new(),
            theme_name: "missing-theme".to_string(),
        };

        let output = md.render("```rust\nlet x = 1;\n```");
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
    fn list_items_respect_configured_width() {
        let md = Markdown::new().with_width(8);
        let output = md.render("- abcdefghijklmnopqrstuvwxyz");

        for line in output.lines() {
            assert!(visible_len(line) <= 8, "{line:?}");
        }
    }

    #[test]
    fn nested_paragraphs_respect_configured_width() {
        let md = Markdown::new().with_width(2);
        let output = md.render("- item\n\n  continuation text");

        for line in output.lines() {
            assert!(visible_len(line) <= 2, "{line:?}");
        }
    }

    #[test]
    fn table_respects_configured_width() {
        let md = Markdown::new().with_width(12);
        let output =
            md.render("| Name | Value |\n| --- | --- |\n| alpha | abcdefghijklmnopqrstuvwxyz |");

        for line in output.lines() {
            assert!(visible_len(line) <= 12, "{line:?}");
        }
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
    fn blockquote_wraps_to_configured_width() {
        let md = Markdown::new().with_width(8);
        let output = md.render("> abcdefghijklmnopqrstuvwxyz");
        let plain = strip_ansi(&output);

        assert_eq!(
            plain
                .lines()
                .map(|line| line.trim_start_matches("│ "))
                .collect::<String>(),
            "abcdefghijklmnopqrstuvwxyz"
        );
        assert!(output.lines().count() > 1);
        for line in output.lines() {
            assert!(visible_len(line) <= 8, "{line:?}");
        }
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
    fn render_element_preserves_heading_style() {
        let md = Markdown::new();
        let el: Element<()> = md.render_element("# Hello");

        let Element::Box(box_el) = el else {
            panic!("expected Box");
        };
        let Element::Text(text) = &box_el.children[0] else {
            panic!("expected heading text");
        };

        assert!(text.style.bold);
        assert_eq!(text.style.fg, Some(Color::Rgb(122, 162, 247)));
        assert!(!text.content.contains('\x1b'));
    }

    #[test]
    fn render_element_preserves_trailing_blank_rows() {
        let md = Markdown::new();
        let el: Element<()> = md.render_element("# Hello");

        let Element::Box(box_el) = el else {
            panic!("expected Box");
        };

        assert_eq!(box_el.children.len(), 2);
        let Element::Text(blank) = &box_el.children[1] else {
            panic!("expected trailing blank text row");
        };
        assert_eq!(blank.content, "");
    }

    #[test]
    fn with_width_wraps() {
        let md = Markdown::new().with_width(10);
        let output = md.render("this is a long sentence that should wrap");
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() > 1);
    }

    #[test]
    fn paragraph_hard_breaks_long_word_by_display_width() {
        let md = Markdown::new().with_width(8);
        let output = md.render("abcdefghijklmnopqrstuvwxyz");
        let plain = strip_ansi(&output);

        assert_eq!(
            plain.lines().collect::<String>(),
            "abcdefghijklmnopqrstuvwxyz"
        );
        for line in output.lines() {
            assert!(visible_len(line) <= 8, "{line:?}");
        }
    }

    #[test]
    fn wrap_text_hard_break_keeps_zero_width_marks_with_base_glyph() {
        let lines = wrap_text("e\u{301}e\u{301}e", 1);

        assert!(lines.iter().all(|line| visible_len(line) <= 1));
        assert_eq!(lines, vec!["e\u{301}", "e\u{301}", "e"]);
    }

    #[test]
    fn wrap_text_hard_break_packs_zero_width_marks_by_display_width() {
        let lines = wrap_text("e\u{301}e\u{301}e", 2);

        assert!(lines.iter().all(|line| visible_len(line) <= 2));
        assert_eq!(lines, vec!["e\u{301}e\u{301}", "e"]);
    }

    #[test]
    fn paragraph_clips_wide_glyphs_at_single_column_width() {
        let md = Markdown::new().with_width(1);
        let output = md.render("中文");

        for line in output.lines() {
            assert!(visible_len(line) <= 1, "{line:?}");
        }
    }

    #[test]
    fn oversized_width_is_clamped() {
        let md = Markdown::new().with_width(usize::MAX);

        assert_eq!(md.width, MAX_MARKDOWN_WIDTH);
        assert!(md.render("hello").contains("hello"));
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
