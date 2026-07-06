use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, strip_ansi, truncate_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_OUTPUT_BLOCK_BODY_LINES: usize = u16::MAX as usize;
const MAX_OUTPUT_BLOCK_INDENT: usize = u16::MAX as usize;

/// Status indicator for an [`OutputBlock`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputStatus {
    #[default]
    Success,
    Error,
    Running,
    Info,
}

/// Compact terminal output block with a status bullet and a tail preview.
///
/// This extracts the common agent/CLI transcript pattern: a colored status
/// marker, a bold action label, muted metadata, and a connector-led output tail
/// that omits earlier noisy lines.
#[derive(Debug, Clone)]
pub struct OutputBlock {
    title: String,
    detail: Option<OutputDetail>,
    status: OutputStatus,
    lines: Vec<String>,
    max_body_lines: usize,
    indent: usize,
    connector: String,
    bullet: String,
    show_omitted_count: bool,
    title_color: Color,
    detail_color: Color,
    success_color: Color,
    error_color: Color,
    running_color: Color,
    info_color: Color,
    body_color: Option<Color>,
    connector_color: Color,
}

impl OutputBlock {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            detail: None,
            status: OutputStatus::Success,
            lines: Vec::new(),
            max_body_lines: 5,
            indent: 2,
            connector: "⎿".to_string(),
            bullet: "•".to_string(),
            show_omitted_count: true,
            title_color: Color::BrightWhite,
            detail_color: Color::BrightBlack,
            success_color: Color::Green,
            error_color: Color::Red,
            running_color: Color::Yellow,
            info_color: Color::Cyan,
            body_color: None,
            connector_color: Color::BrightBlack,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(OutputDetail::Plain(detail.into()));
        self
    }

    pub fn styled_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(OutputDetail::Styled(detail.into()));
        self
    }

    pub fn status(mut self, status: OutputStatus) -> Self {
        self.status = status;
        self
    }

    pub fn success(self) -> Self {
        self.status(OutputStatus::Success)
    }

    pub fn error(self) -> Self {
        self.status(OutputStatus::Error)
    }

    pub fn running(self) -> Self {
        self.status(OutputStatus::Running)
    }

    pub fn info(self) -> Self {
        self.status(OutputStatus::Info)
    }

    /// Set body lines exactly as provided.
    pub fn lines(mut self, lines: Vec<impl Into<String>>) -> Self {
        self.lines = lines.into_iter().map(Into::into).collect();
        self
    }

    /// Set body lines from text, dropping blank rows for compact transcripts.
    pub fn text(mut self, text: impl AsRef<str>) -> Self {
        self.lines = text
            .as_ref()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(str::to_string)
            .collect();
        self
    }

    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.lines.push(line.into());
        self
    }

    pub fn add_line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    pub fn max_body_lines(mut self, max_body_lines: usize) -> Self {
        self.max_body_lines = max_body_lines.clamp(1, MAX_OUTPUT_BLOCK_BODY_LINES);
        self
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent.min(MAX_OUTPUT_BLOCK_INDENT);
        self
    }

    pub fn connector(mut self, connector: impl Into<String>) -> Self {
        self.connector = connector.into();
        self
    }

    pub fn bullet(mut self, bullet: impl Into<String>) -> Self {
        self.bullet = bullet.into();
        self
    }

    pub fn show_omitted_count(mut self, enabled: bool) -> Self {
        self.show_omitted_count = enabled;
        self
    }

    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    pub fn detail_color(mut self, color: Color) -> Self {
        self.detail_color = color;
        self
    }

    pub fn status_colors(
        mut self,
        success: Color,
        error: Color,
        running: Color,
        info: Color,
    ) -> Self {
        self.success_color = success;
        self.error_color = error;
        self.running_color = running;
        self.info_color = info;
        self
    }

    pub fn body_color(mut self, color: Color) -> Self {
        self.body_color = Some(color);
        self
    }

    pub fn connector_color(mut self, color: Color) -> Self {
        self.connector_color = color;
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.title_color = theme.color(ThemeRole::Primary);
        self.detail_color = theme.color(ThemeRole::Muted);
        self.success_color = theme.color(ThemeRole::Success);
        self.error_color = theme.color(ThemeRole::Error);
        self.running_color = theme.color(ThemeRole::Warning);
        self.info_color = theme.color(ThemeRole::Info);
        self.body_color = None;
        self.connector_color = theme.color(ThemeRole::Border);
        self
    }

    pub fn title_value(&self) -> &str {
        &self.title
    }

    pub fn detail_value(&self) -> Option<&str> {
        self.detail.as_ref().map(OutputDetail::as_str)
    }

    pub fn lines_value(&self) -> &[String] {
        &self.lines
    }

    pub fn status_value(&self) -> OutputStatus {
        self.status
    }

    pub fn view(&self, width: u16) -> String {
        let width = width as usize;
        if width == 0 {
            return String::new();
        }

        let mut lines = Vec::with_capacity(1 + self.max_body_lines + 1);
        lines.push(self.render_header(width));
        lines.extend(self.render_body(width));
        lines
            .into_iter()
            .map(|line| fit_visible(&line, width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children = Vec::new();
        let mut header = TextElement::new(self.plain_header()).fg(self.title_color);
        header = header.bold();
        children.push(Element::Text(header));

        for (index, row) in self.body_rows().into_iter().enumerate() {
            let prefix = if index == 0 && !row.is_continuation {
                self.body_prefix_plain()
            } else {
                self.body_continuation_plain()
            };
            children.push(Element::Box(
                BoxElement::new()
                    .direction(FlexDirection::Row)
                    .child(Element::Text(
                        TextElement::new(prefix).fg(self.connector_color),
                    ))
                    .child(Element::Text(TextElement::new(row.text).fg(
                        if row.omitted {
                            self.detail_color
                        } else {
                            self.effective_body_color()
                        },
                    ))),
            ));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_header(&self, width: usize) -> String {
        let prefix = " ".repeat(self.header_indent_for_width(width));
        let bullet = Style::new()
            .fg(self.status_color())
            .bold()
            .render(&self.bullet);
        let title = Style::new().fg(self.title_color).bold().render(&self.title);
        let mut raw = format!("{prefix}{bullet} {title}");
        if let Some(detail) = self.detail.as_ref().filter(|detail| !detail.is_empty()) {
            let used = visible_len(&raw);
            let available = width.saturating_sub(used.saturating_add(1));
            raw.push(' ');
            raw.push_str(&self.render_detail(detail, available));
        }
        raw
    }

    fn render_body(&self, width: usize) -> Vec<String> {
        let body_width = self.body_text_width(width);
        self.body_rows()
            .into_iter()
            .enumerate()
            .map(|(index, row)| {
                let prefix = if index == 0 && !row.is_continuation {
                    self.body_prefix_styled_for_width(width)
                } else {
                    self.body_continuation_plain_for_width(width)
                };
                let color = if row.omitted {
                    self.detail_color
                } else {
                    self.effective_body_color()
                };
                let text = truncate_visible(&row.text, body_width);
                format!("{prefix}{}", Style::new().fg(color).render(&text))
            })
            .collect()
    }

    fn body_rows(&self) -> Vec<BodyRow> {
        if self.lines.is_empty() {
            return Vec::new();
        }

        let start = self.lines.len().saturating_sub(self.max_body_lines);
        let mut rows = Vec::new();
        if start > 0 && self.show_omitted_count {
            rows.push(BodyRow {
                text: format!("… +{start} earlier lines"),
                omitted: true,
                is_continuation: false,
            });
        }

        for (offset, line) in self.lines.iter().skip(start).enumerate() {
            rows.push(BodyRow {
                text: line.trim_end_matches('\r').to_string(),
                omitted: false,
                is_continuation: start > 0 && self.show_omitted_count || offset > 0,
            });
        }
        rows
    }

    fn plain_header(&self) -> String {
        match self.detail.as_ref().filter(|detail| !detail.is_empty()) {
            Some(detail) => format!(
                "{}{} {} {}",
                " ".repeat(self.indent_for_element()),
                self.bullet,
                self.title,
                strip_ansi(detail.as_str())
            ),
            None => format!(
                "{}{} {}",
                " ".repeat(self.indent_for_element()),
                self.bullet,
                self.title
            ),
        }
    }

    fn body_prefix_plain(&self) -> String {
        self.body_prefix_plain_for_indent(self.indent_for_element())
    }

    fn body_prefix_styled_for_width(&self, width: usize) -> String {
        format!(
            "{}  {}  ",
            " ".repeat(self.body_indent_for_width(width)),
            Style::new()
                .fg(self.connector_color)
                .render(&self.connector)
        )
    }

    fn body_continuation_plain(&self) -> String {
        " ".repeat(visible_len(&self.body_prefix_plain()))
    }

    fn body_continuation_plain_for_width(&self, width: usize) -> String {
        " ".repeat(visible_len(&self.body_prefix_plain_for_width(width)))
    }

    fn body_text_width(&self, width: usize) -> usize {
        width
            .saturating_sub(visible_len(&self.body_prefix_plain_for_width(width)))
            .max(1)
    }

    fn header_indent_for_width(&self, width: usize) -> usize {
        let fixed_width = visible_len(&self.bullet).saturating_add(1);
        self.indent
            .min(width.saturating_sub(fixed_width))
            .min(MAX_OUTPUT_BLOCK_INDENT)
    }

    fn body_indent_for_width(&self, width: usize) -> usize {
        let fixed_width = visible_len(&self.connector).saturating_add(5);
        self.indent
            .min(width.saturating_sub(fixed_width))
            .min(MAX_OUTPUT_BLOCK_INDENT)
    }

    fn body_prefix_plain_for_width(&self, width: usize) -> String {
        self.body_prefix_plain_for_indent(self.body_indent_for_width(width))
    }

    fn body_prefix_plain_for_indent(&self, indent: usize) -> String {
        format!("{}  {}  ", " ".repeat(indent), self.connector)
    }

    fn indent_for_element(&self) -> usize {
        self.indent.min(MAX_OUTPUT_BLOCK_INDENT)
    }

    fn status_color(&self) -> Color {
        match self.status {
            OutputStatus::Success => self.success_color,
            OutputStatus::Error => self.error_color,
            OutputStatus::Running => self.running_color,
            OutputStatus::Info => self.info_color,
        }
    }

    fn effective_body_color(&self) -> Color {
        self.body_color.unwrap_or(match self.status {
            OutputStatus::Success | OutputStatus::Info => self.detail_color,
            OutputStatus::Error => self.error_color,
            OutputStatus::Running => self.running_color,
        })
    }

    fn render_detail(&self, detail: &OutputDetail, width: usize) -> String {
        match detail {
            OutputDetail::Plain(value) => Style::new()
                .fg(self.detail_color)
                .render(&truncate_visible(value, width)),
            OutputDetail::Styled(value) => truncate_visible(value, width),
        }
    }
}

impl Default for OutputBlock {
    fn default() -> Self {
        Self::new("")
    }
}

#[derive(Debug, Clone)]
struct BodyRow {
    text: String,
    omitted: bool,
    is_continuation: bool,
}

#[derive(Debug, Clone)]
enum OutputDetail {
    Plain(String),
    Styled(String),
}

impl OutputDetail {
    fn as_str(&self) -> &str {
        match self {
            Self::Plain(value) | Self::Styled(value) => value,
        }
    }

    fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn renders_header_and_tail_with_omitted_count() {
        let block = OutputBlock::new("Ran")
            .detail("npm test")
            .text("one\ntwo\nthree\nfour\nfive\nsix")
            .max_body_lines(3);
        let rendered = block.view(42);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("• Ran npm test"));
        assert!(plain.contains("⎿  … +3 earlier lines"));
        assert!(!plain.contains("one"));
        assert!(plain.contains("four"));
        assert!(plain.contains("six"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 42, "{line:?}");
        }
    }

    #[test]
    fn empty_body_renders_only_header() {
        let rendered = OutputBlock::new("Read").detail("src/main.rs").view(30);
        let plain = strip_ansi(&rendered);

        assert_eq!(plain.lines().count(), 1);
        assert!(plain.contains("Read src/main.rs"));
    }

    #[test]
    fn error_status_colors_bullet_and_body() {
        let rendered = OutputBlock::new("Ran").error().line("failed").view(24);

        assert!(rendered.contains("\x1b[31"));
        assert!(strip_ansi(&rendered).contains("failed"));
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let block = OutputBlock::new("Ran")
            .body_color(Color::Red)
            .with_theme(&theme);

        assert_eq!(block.title_color, theme.color(ThemeRole::Primary));
        assert_eq!(block.detail_color, theme.color(ThemeRole::Muted));
        assert_eq!(block.success_color, theme.color(ThemeRole::Success));
        assert_eq!(block.error_color, theme.color(ThemeRole::Error));
        assert_eq!(block.running_color, theme.color(ThemeRole::Warning));
        assert_eq!(block.info_color, theme.color(ThemeRole::Info));
        assert_eq!(block.body_color, None);
        assert_eq!(block.connector_color, theme.color(ThemeRole::Border));
    }

    #[test]
    fn preserves_styled_detail() {
        let detail = format!(
            "{} {}",
            Style::new().fg(Color::Cyan).bold().render("cargo"),
            Style::new().fg(Color::Yellow).render("test")
        );
        let rendered = OutputBlock::new("Ran")
            .styled_detail(detail)
            .line("ok")
            .view(36);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("• Ran cargo test"));
        assert!(rendered.contains("\x1b[1;36mcargo\x1b[0m"));
        assert!(rendered.contains("\x1b[33mtest\x1b[0m"));
        assert!(rendered.lines().all(|line| visible_len(line) == 36));
    }

    #[test]
    fn text_drops_blank_rows_for_compact_output() {
        let rendered = OutputBlock::new("Ran").text("one\n\n  \ntwo").view(24);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("one"));
        assert!(plain.contains("two"));
        assert_eq!(plain.lines().count(), 3);
    }

    #[test]
    fn truncates_cjk_detail_and_body_to_width() {
        let rendered = OutputBlock::new("Read")
            .detail("中文测试内容文件路径.rs")
            .line("输出包含中文测试内容 and a long suffix")
            .view(24);

        for line in rendered.lines() {
            assert_eq!(visible_len(line), 24, "{line:?}");
        }
        assert!(strip_ansi(&rendered).contains("中文"));
    }

    #[test]
    fn zero_width_renders_empty_string() {
        assert_eq!(OutputBlock::new("Ran").line("x").view(0), "");
    }

    #[test]
    fn oversized_indent_is_clamped_to_render_width() {
        let block = OutputBlock::new("Ran")
            .detail("npm test")
            .indent(usize::MAX)
            .line("completed");
        let rendered = block.view(8);

        assert_eq!(block.indent, MAX_OUTPUT_BLOCK_INDENT);
        assert_eq!(block.header_indent_for_width(8), 6);
        assert_eq!(block.body_indent_for_width(8), 2);
        assert_eq!(block.body_text_width(8), 1);
        assert!(rendered.lines().all(|line| visible_len(line) == 8));

        let body = block.render_body(8);
        let plain_body = strip_ansi(&body[0]);
        assert!(plain_body.starts_with("    ⎿  "));

        let Element::Box(column) = block.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Text(header) = &column.children[0] else {
            panic!("expected header text");
        };
        assert_eq!(
            visible_len(&header.content),
            MAX_OUTPUT_BLOCK_INDENT + visible_len("• Ran npm test")
        );
    }

    #[test]
    fn oversized_body_line_limit_is_clamped() {
        let block = OutputBlock::new("Ran")
            .max_body_lines(usize::MAX)
            .text("one\ntwo");
        let rendered = block.view(8);

        assert_eq!(block.max_body_lines, MAX_OUTPUT_BLOCK_BODY_LINES);
        assert!(rendered.lines().all(|line| visible_len(line) == 8));
        assert_eq!(block.body_rows().len(), 2);
    }

    #[test]
    fn element_produces_column() {
        let el: Element<()> = OutputBlock::new("Ran").line("ok").element();

        match el {
            Element::Box(column) => {
                assert_eq!(column.style.flex_direction, FlexDirection::Column);
                assert_eq!(column.children.len(), 2);
            }
            _ => panic!("expected Box"),
        }
    }
}
