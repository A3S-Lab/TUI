//! Markdown-to-terminal renderer.
//!
//! Supports CommonMark via comrak. Code highlighting is enabled by the
//! `syntax-highlighting` crate feature.

#[cfg(feature = "syntax-highlighting")]
use std::sync::{Arc, OnceLock};

use comrak::nodes::{AstNode, NodeValue};
use comrak::{parse_document, Arena, Options};
#[cfg(feature = "syntax-highlighting")]
use syntect::easy::HighlightLines;
#[cfg(feature = "syntax-highlighting")]
use syntect::highlighting::{Style as SynStyle, ThemeSet};
#[cfg(feature = "syntax-highlighting")]
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::style::{
    split_lines_preserving_trailing_blank, truncate_visible, visible_len, Color, Style,
};

const MAX_MARKDOWN_WIDTH: usize = u16::MAX as usize;

/// Match Codex's syntax-highlighting guardrails so pathological model output
/// cannot monopolize the render loop. Oversized blocks remain plain text.
#[cfg(feature = "syntax-highlighting")]
const MAX_HIGHLIGHT_BYTES: usize = 512 * 1024;
#[cfg(feature = "syntax-highlighting")]
const MAX_HIGHLIGHT_LINES: usize = 10_000;

mod ansi;
mod table;

use ansi::wrap_styled_text;

/// Background active on the final rendered cell of an ANSI-styled line.
/// Useful when adjacent chrome must visually continue a full-row surface.
pub fn trailing_ansi_background(line: &str) -> Option<Color> {
    ansi::trailing_background(line)
}
pub(crate) use ansi::{rendered_markdown_element, split_rendered_lines};

#[cfg(feature = "syntax-highlighting")]
struct DefaultSyntaxAssets {
    syntax_set: Arc<SyntaxSet>,
    theme_set: Arc<ThemeSet>,
}

#[cfg(feature = "syntax-highlighting")]
fn default_syntax_assets() -> &'static DefaultSyntaxAssets {
    static DEFAULTS: OnceLock<DefaultSyntaxAssets> = OnceLock::new();
    DEFAULTS.get_or_init(|| DefaultSyntaxAssets {
        syntax_set: Arc::new(SyntaxSet::load_defaults_newlines()),
        theme_set: Arc::new(ThemeSet::load_defaults()),
    })
}

pub struct Markdown {
    width: usize,
    #[cfg(feature = "syntax-highlighting")]
    syntax_set: Arc<SyntaxSet>,
    #[cfg(feature = "syntax-highlighting")]
    theme_set: Arc<ThemeSet>,
    #[cfg(feature = "syntax-highlighting")]
    theme_name: String,
}

/// Source-aware Markdown output used by clients that need to preserve lines
/// whose whitespace is semantically significant.
#[derive(Debug, Clone)]
pub struct RenderedMarkdown {
    text: String,
    non_wrapping_line_indices: Vec<usize>,
    line_metadata: Vec<RenderedLineMetadata>,
}

/// One-based source-line range that produced a rendered terminal row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLineRange {
    start: usize,
    end: usize,
}

impl SourceLineRange {
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn contains(&self, line: usize) -> bool {
        self.start <= line && line <= self.end
    }
}

/// Source and layout semantics for one rendered Markdown row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderedLineMetadata {
    source: Option<SourceLineRange>,
    non_wrapping: bool,
    table: bool,
}

impl RenderedLineMetadata {
    pub fn source(&self) -> Option<SourceLineRange> {
        self.source
    }

    pub fn is_non_wrapping(&self) -> bool {
        self.non_wrapping
    }

    pub fn is_table(&self) -> bool {
        self.table
    }
}

impl RenderedMarkdown {
    /// Return the rendered ANSI text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Consume the metadata wrapper and return the rendered ANSI text.
    pub fn into_text(self) -> String {
        self.text
    }

    /// Zero-based rendered rows that must not be reflowed by a downstream
    /// viewport pass. Today these are code-block rows, matching Codex CLI's
    /// copy/paste-preserving behavior.
    pub fn non_wrapping_line_indices(&self) -> &[usize] {
        &self.non_wrapping_line_indices
    }

    /// Metadata aligned one-for-one with `self.as_str().lines()`.
    pub fn line_metadata(&self) -> &[RenderedLineMetadata] {
        &self.line_metadata
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MarkdownLineKind {
    Normal,
    NonWrapping,
}

#[derive(Debug)]
struct MarkdownLine {
    text: String,
    kind: MarkdownLineKind,
    source: Option<SourceLineRange>,
    table: bool,
}

struct ListItemMarker<'a> {
    text: &'a str,
    width: usize,
    dim: bool,
}

impl MarkdownLine {
    fn normal(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: MarkdownLineKind::Normal,
            source: None,
            table: false,
        }
    }

    fn non_wrapping(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: MarkdownLineKind::NonWrapping,
            source: None,
            table: false,
        }
    }
}

impl Markdown {
    pub fn new() -> Self {
        #[cfg(feature = "syntax-highlighting")]
        let defaults = default_syntax_assets();
        Self {
            width: 80,
            #[cfg(feature = "syntax-highlighting")]
            syntax_set: Arc::clone(&defaults.syntax_set),
            #[cfg(feature = "syntax-highlighting")]
            theme_set: Arc::clone(&defaults.theme_set),
            #[cfg(feature = "syntax-highlighting")]
            theme_name: "base16-eighties.dark".to_string(),
        }
    }

    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width.min(MAX_MARKDOWN_WIDTH);
        self
    }

    pub fn with_theme(self, name: &str) -> Self {
        #[cfg(feature = "syntax-highlighting")]
        {
            let mut markdown = self;
            markdown.theme_name = name.to_string();
            markdown
        }
        #[cfg(not(feature = "syntax-highlighting"))]
        {
            let _ = name;
            self
        }
    }

    pub fn render(&self, input: &str) -> String {
        self.render_with_metadata(input).into_text()
    }

    /// Render Markdown while retaining source-aware row layout metadata.
    pub fn render_with_metadata(&self, input: &str) -> RenderedMarkdown {
        let arena = Arena::new();
        let mut options = Options::default();
        options.extension.table = true;
        options.extension.strikethrough = true;
        options.extension.autolink = true;
        options.extension.tasklist = true;
        let root = parse_document(&arena, input, &options);

        let mut output = Vec::new();
        self.render_node(root, &mut output, 0, self.width);
        let text = output
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let rendered_line_count = text.lines().count();
        let non_wrapping_line_indices = output
            .iter()
            .take(rendered_line_count)
            .enumerate()
            .filter_map(|(index, line)| {
                (line.kind == MarkdownLineKind::NonWrapping).then_some(index)
            })
            .collect();
        let line_metadata = output
            .iter()
            .take(rendered_line_count)
            .map(|line| RenderedLineMetadata {
                source: line.source,
                non_wrapping: line.kind == MarkdownLineKind::NonWrapping,
                table: line.table,
            })
            .collect();
        RenderedMarkdown {
            text,
            non_wrapping_line_indices,
            line_metadata,
        }
    }

    fn render_node<'a>(
        &self,
        node: &'a AstNode<'a>,
        output: &mut Vec<MarkdownLine>,
        indent: usize,
        width: usize,
    ) {
        let output_start = output.len();
        let (source, table_node) = {
            let data = node.data.borrow();
            (
                SourceLineRange {
                    start: data.sourcepos.start.line,
                    end: data.sourcepos.end.line,
                },
                matches!(data.value, NodeValue::Table(_)),
            )
        };
        match &node.data.borrow().value {
            NodeValue::Document => {
                for child in node.children() {
                    self.render_node(child, output, indent, width);
                }
            }
            NodeValue::Heading(heading) => {
                let text = self.collect_inline(node);
                let label = format!("{} {text}", "#".repeat(heading.level as usize));
                for line in wrap_text(&label, width.saturating_sub(indent)) {
                    let styled = Style::new()
                        .bold()
                        .fg(heading_color(heading.level))
                        .render(&line);
                    output.push(MarkdownLine::normal(fit_markdown_line(
                        format!("{}{styled}", " ".repeat(indent)),
                        width,
                    )));
                }
                output.push(MarkdownLine::normal(String::new()));
            }
            NodeValue::Paragraph => {
                let text = self.collect_inline(node);
                let wrapped = wrap_text(&text, width.saturating_sub(indent));
                for line in wrapped {
                    output.push(MarkdownLine::normal(fit_markdown_line(
                        format!("{}{}", " ".repeat(indent), line),
                        width,
                    )));
                }
                // No trailing blank line — keeps chat messages tight (a model that
                // writes one sentence per paragraph would otherwise be double-spaced).
            }
            NodeValue::CodeBlock(cb) => {
                let lang = cb.info.split([',', ' ', '\t']).next().unwrap_or_default();
                let code = cb.literal.clone();
                let highlighted = self.highlight_code(&code, lang);
                let code = highlighted.strip_suffix('\n').unwrap_or(&highlighted);

                if !code.is_empty() {
                    if output.last().is_some_and(|line| !line.text.is_empty()) {
                        output.push(MarkdownLine::normal(String::new()));
                    }
                    let code_indent = indent.saturating_add(usize::from(!cb.fenced) * 4);
                    let indent = " ".repeat(code_indent);
                    for line in split_lines_preserving_trailing_blank(code) {
                        output.push(MarkdownLine::non_wrapping(format!("{indent}{line}")));
                    }
                    // This trailing separator is invisible at end-of-document,
                    // but gives a following block Codex's normal vertical gap.
                    // It is deliberately not a code row, so an unterminated
                    // fence never exposes a speculative footer to scrollback.
                    output.push(MarkdownLine::normal(String::new()));
                }
            }
            NodeValue::List(_) => {
                for child in node.children() {
                    self.render_node(child, output, indent, width);
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
                // label, while subsequent blocks inherit the exact marker
                // continuation indent.
                self.render_item_body(
                    node,
                    output,
                    indent,
                    width,
                    ListItemMarker {
                        text: &mark,
                        width: 1,
                        dim: checked.is_none(),
                    },
                );
            }
            NodeValue::Item(item) => {
                let bullet = if item.list_type == comrak::nodes::ListType::Ordered {
                    format!("{}.", item.start)
                } else {
                    "-".to_string()
                };
                let bw = visible_len(&bullet);
                let bullet_style = Style::new().fg(Color::Rgb(122, 162, 247)).render(&bullet);
                // The item's first paragraph is its bulleted label; every other
                // block child (nested list, code block, extra paragraph) renders
                // through the normal recursive path. (Flattening the
                // whole subtree into the label is what rendered nested lists twice.)
                self.render_item_body(
                    node,
                    output,
                    indent,
                    width,
                    ListItemMarker {
                        text: &bullet_style,
                        width: bw,
                        dim: false,
                    },
                );
            }
            NodeValue::BlockQuote => {
                let inner_width = if width == 0 {
                    0
                } else {
                    width.saturating_sub(2)
                };
                let mut quoted = Vec::new();
                for child in node.children() {
                    self.render_node(child, &mut quoted, indent, inner_width);
                }
                for line in quoted {
                    output.push(prefix_blockquote_line(line, indent, width));
                }
            }
            NodeValue::ThematicBreak => {
                let rule = Style::new()
                    .fg(Color::BrightBlack)
                    .render(&"─".repeat(width));
                output.push(MarkdownLine::normal(rule));
                output.push(MarkdownLine::normal(String::new()));
            }
            NodeValue::Table(table) => self.render_table(node, table, output, indent, width),
            _ => {
                for child in node.children() {
                    self.render_node(child, output, indent, width);
                }
            }
        }
        for line in &mut output[output_start..] {
            if line.source.is_none() {
                line.source = Some(source);
                line.table = table_node;
            }
        }
    }

    /// Push a list item, wrapping long text with a hanging indent so it lines
    /// up under the first character after the bullet. `bullet` is pre-styled;
    /// `bw` is its visible width.
    fn push_list_item(
        &self,
        output: &mut Vec<MarkdownLine>,
        indent: usize,
        width: usize,
        bullet: &str,
        bw: usize,
        text: &str,
    ) {
        let prefix_w = indent + bw + 1;
        let indent_text = " ".repeat(indent);
        let hang = " ".repeat(prefix_w);
        if width != 0 && prefix_w >= width && !text.is_empty() {
            output.push(MarkdownLine::normal(fit_markdown_line(
                format!("{indent_text}{bullet}"),
                width,
            )));
            output.extend(
                wrap_text(text, width)
                    .into_iter()
                    .map(|line| MarkdownLine::normal(fit_markdown_line(line, width))),
            );
            return;
        }

        let avail = if width == 0 {
            0
        } else {
            width.saturating_sub(prefix_w)
        };
        let wrapped = wrap_text(text, avail);
        for (i, line) in wrapped.iter().enumerate() {
            if i == 0 {
                output.push(MarkdownLine::normal(fit_markdown_line(
                    format!("{indent_text}{bullet} {line}"),
                    width,
                )));
            } else {
                output.push(MarkdownLine::normal(fit_markdown_line(
                    format!("{hang}{line}"),
                    width,
                )));
            }
        }
    }

    /// Render a list-item body: the item's first paragraph becomes the bulleted
    /// label; every other block child (nested list, code block, extra paragraph)
    /// renders through the normal recursive path at its structural indent. This is the
    /// single source of truth for item layout — no subtree flattening, so a
    /// nested list renders once (as a sub-list), never also jammed into the label.
    /// `dim` greys the label (unchecked task items).
    fn render_item_body<'a>(
        &self,
        node: &'a AstNode<'a>,
        output: &mut Vec<MarkdownLine>,
        indent: usize,
        width: usize,
        marker: ListItemMarker<'_>,
    ) {
        let mut labeled = false;
        for child in node.children() {
            if !labeled && matches!(&child.data.borrow().value, NodeValue::Paragraph) {
                let text = self.collect_inline(child);
                let text = if marker.dim {
                    Style::new().fg(Color::BrightBlack).render(&text)
                } else {
                    text
                };
                self.push_list_item(output, indent, width, marker.text, marker.width, &text);
                labeled = true;
            } else {
                let child_indent = if matches!(&child.data.borrow().value, NodeValue::List(_)) {
                    indent.saturating_add(4)
                } else {
                    indent.saturating_add(marker.width).saturating_add(1)
                };
                self.render_node(child, output, child_indent, width);
            }
        }
        // An item that starts with a sub-list (no leading paragraph) still gets a
        // bullet so the nesting reads correctly.
        if !labeled {
            self.push_list_item(output, indent, width, marker.text, marker.width, "");
        }
    }

    fn collect_inline<'a>(&self, node: &'a AstNode<'a>) -> String {
        let mut parts = Vec::new();
        for child in node.children() {
            self.collect_inline_node(child, &mut parts);
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
                let styled = format!("\x1b[48;5;236m\x1b[33m {} \x1b[39;49m", code_text);
                parts.push(styled);
            }
            NodeValue::Strong => {
                let inner = self.collect_inline(node);
                parts.push(format!("\x1b[1m{inner}\x1b[22m"));
            }
            NodeValue::Emph => {
                let inner = self.collect_inline(node);
                parts.push(format!("\x1b[3m{inner}\x1b[23m"));
            }
            NodeValue::Strikethrough => {
                let inner = self.collect_inline(node);
                parts.push(format!("\x1b[9m{inner}\x1b[29m"));
            }
            NodeValue::Link(link) => {
                let text = self.collect_inline(node);
                let (label, suffix, url) = split_autolink_suffix(&text, &link.url);
                let url = url
                    .chars()
                    .filter(|ch| !ch.is_control())
                    .collect::<String>();
                parts.push(format!(
                    "\x1b]8;;{url}\x1b\\\x1b[4;34m{label}\x1b[24;39m\x1b]8;;\x1b\\{suffix}"
                ));
            }
            NodeValue::SoftBreak | NodeValue::LineBreak => parts.push("\n".to_string()),
            _ => {
                for child in node.children() {
                    self.collect_inline_node(child, parts);
                }
            }
        }
    }

    #[cfg(feature = "syntax-highlighting")]
    fn highlight_code(&self, code: &str, lang: &str) -> String {
        let exceeds_limits =
            code.len() > MAX_HIGHLIGHT_BYTES || code.lines().count() > MAX_HIGHLIGHT_LINES;
        let normalized = normalize_code_newlines(code);
        let code = normalized.as_ref();
        if code.is_empty() || exceeds_limits {
            return code.to_string();
        }

        // Unknown language tags must stay completely unstyled. Falling back to
        // syntect's Plain Text grammar still applies the theme foreground and
        // makes arbitrary fence labels look like successful highlighting.
        let Some(syntax) = find_syntax(&self.syntax_set, lang) else {
            return code.to_string();
        };

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
            let Ok(ranges) = h.highlight_line(line, &self.syntax_set) else {
                return code.to_string();
            };
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
        normalize_code_newlines(code).into_owned()
    }
}

fn normalize_code_newlines(code: &str) -> std::borrow::Cow<'_, str> {
    if code.contains('\r') {
        std::borrow::Cow::Owned(code.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        std::borrow::Cow::Borrowed(code)
    }
}

#[cfg(feature = "syntax-highlighting")]
fn find_syntax<'a>(syntax_set: &'a SyntaxSet, lang: &str) -> Option<&'a SyntaxReference> {
    let lang = lang.trim();
    if lang.is_empty() {
        return None;
    }

    let normalized = lang.to_ascii_lowercase();
    let patched = match normalized.as_str() {
        "csharp" | "c-sharp" => "cs",
        "cppm" | "cxxm" | "ixx" => "cpp",
        "golang" => "go",
        "python3" => "python",
        "shell" => "bash",
        _ => lang,
    };

    syntax_set
        .find_syntax_by_token(patched)
        .or_else(|| syntax_set.find_syntax_by_name(patched))
        .or_else(|| {
            let lower = patched.to_ascii_lowercase();
            syntax_set
                .syntaxes()
                .iter()
                .find(|syntax| syntax.name.to_ascii_lowercase() == lower)
        })
        .or_else(|| syntax_set.find_syntax_by_extension(lang))
}

fn split_autolink_suffix<'a>(text: &'a str, url: &'a str) -> (&'a str, &'a str, &'a str) {
    if text != url || !url.starts_with("http://") && !url.starts_with("https://") {
        return (text, "", url);
    }

    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    let mut boundary = url.len();
    for (index, ch) in url.char_indices() {
        match ch {
            '(' => round += 1,
            '[' => square += 1,
            '{' => curly += 1,
            ')' if round == 0 => {
                boundary = index;
                break;
            }
            ']' if square == 0 => {
                boundary = index;
                break;
            }
            '}' if curly == 0 => {
                boundary = index;
                break;
            }
            ')' => round -= 1,
            ']' => square -= 1,
            '}' => curly -= 1,
            _ => {}
        }
    }

    while boundary > 0 {
        let Some(ch) = url[..boundary].chars().next_back() else {
            break;
        };
        if matches!(
            ch,
            '.' | ',' | ';' | ':' | '!' | '?' | '。' | '，' | '；' | '：' | '！' | '？'
        ) {
            boundary -= ch.len_utf8();
        } else {
            break;
        }
    }

    if boundary == url.len() || boundary == 0 {
        (text, "", url)
    } else {
        (&text[..boundary], &text[boundary..], &url[..boundary])
    }
}

impl Default for Markdown {
    fn default() -> Self {
        Self::new()
    }
}

/// Add one structural quote level without flattening the rendered child block.
/// The marker is inserted after the containing list's base indentation, so
/// both `> - item` and `- item / > quote` retain their compound prefixes.
fn prefix_blockquote_line(mut line: MarkdownLine, indent: usize, width: usize) -> MarkdownLine {
    if line.text.is_empty() {
        return line;
    }

    let bar = Style::new().fg(Color::BrightBlack).render("│");
    if width == 1 {
        line.text = bar;
        return line;
    }
    if width == 2 {
        line.text = format!("{bar} ");
        return line;
    }

    let insertion = line
        .text
        .as_bytes()
        .iter()
        .take(indent)
        .take_while(|byte| **byte == b' ')
        .count();
    line.text.insert_str(insertion, &format!("{bar} "));
    line
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

fn fit_markdown_line(line: String, width: usize) -> String {
    if width == 0 {
        line
    } else {
        truncate_visible(&line, width)
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return text.split('\n').map(str::to_string).collect();
    }

    let mut lines = Vec::new();
    for source_line in text.split('\n') {
        if source_line.is_empty() {
            lines.push(String::new());
        } else {
            lines.extend(wrap_styled_text(source_line, width));
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

#[cfg(test)]
mod tests;
