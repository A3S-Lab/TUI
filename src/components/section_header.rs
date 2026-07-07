use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, Color, Style};

const MAX_SECTION_HEADER_INDENT: usize = u16::MAX as usize;
const MAX_SECTION_HEADER_LINES: usize = u16::MAX as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionHeaderLineKind {
    Metadata,
    Muted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SectionHeaderLine {
    value: String,
    kind: SectionHeaderLineKind,
}

/// Compact heading block for terminal panels and focus views.
///
/// `SectionHeader` renders a bold title, optional muted metadata lines, and an
/// optional separator after the header. It centralizes the common "title,
/// status, divider" pattern used by terminal dashboards while keeping every
/// line display-width safe.
#[derive(Debug, Clone)]
pub struct SectionHeader {
    title: Option<String>,
    lines: Vec<SectionHeaderLine>,
    show_separator: bool,
    fill_height: bool,
    indent: usize,
    max_lines: Option<usize>,
    title_color: Color,
    metadata_color: Color,
    muted_color: Color,
    separator_color: Color,
}

impl SectionHeader {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            lines: Vec::new(),
            show_separator: true,
            fill_height: false,
            indent: 1,
            max_lines: None,
            title_color: Color::Cyan,
            metadata_color: Color::BrightBlack,
            muted_color: Color::BrightBlack,
            separator_color: Color::BrightBlack,
        }
    }

    pub fn without_title() -> Self {
        Self {
            title: None,
            ..Self::new("")
        }
    }

    pub fn metadata(mut self, value: impl Into<String>) -> Self {
        self.push_line(value, SectionHeaderLineKind::Metadata);
        self
    }

    pub fn muted(mut self, value: impl Into<String>) -> Self {
        self.push_line(value, SectionHeaderLineKind::Muted);
        self
    }

    pub fn show_separator(mut self, enabled: bool) -> Self {
        self.show_separator = enabled;
        self
    }

    pub fn fill_height(mut self, enabled: bool) -> Self {
        self.fill_height = enabled;
        self
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent.min(MAX_SECTION_HEADER_INDENT);
        self
    }

    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = Some(max_lines.clamp(1, MAX_SECTION_HEADER_LINES));
        self
    }

    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    pub fn metadata_color(mut self, color: Color) -> Self {
        self.metadata_color = color;
        self
    }

    pub fn muted_color(mut self, color: Color) -> Self {
        self.muted_color = color;
        self
    }

    pub fn separator_color(mut self, color: Color) -> Self {
        self.separator_color = color;
        self
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        let mut lines = self.render_lines(width, height);
        lines.truncate(height);
        if self.fill_height {
            while lines.len() < height {
                lines.push(String::new());
            }
        }

        lines
            .into_iter()
            .map(|line| fit_visible(&line, width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut children = Vec::new();
        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            children.push(Element::Text(
                TextElement::new(title).fg(self.title_color).bold(),
            ));
        }
        for line in self.visible_lines(usize::MAX) {
            children.push(Element::Text(self.line_element(line)));
        }
        if self.show_separator {
            children.push(Element::Text(
                TextElement::new("─").fg(self.separator_color),
            ));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    pub fn element_with_height<Msg>(&self, height: usize) -> Element<Msg> {
        let mut children = Vec::new();
        if height == 0 {
            return Element::Box(BoxElement::new().direction(FlexDirection::Column));
        }

        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            children.push(Element::Text(
                TextElement::new(title).fg(self.title_color).bold(),
            ));
        }
        let reserved_separator = usize::from(self.show_separator);
        let available = height
            .saturating_sub(children.len())
            .saturating_sub(reserved_separator);
        for line in self.visible_lines(available) {
            children.push(Element::Text(self.line_element(line)));
        }
        if self.show_separator && children.len() < height {
            children.push(Element::Text(
                TextElement::new("─").fg(self.separator_color),
            ));
        }
        children.truncate(height);

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn push_line(&mut self, value: impl Into<String>, kind: SectionHeaderLineKind) {
        let value = value.into();
        if !value.is_empty() {
            self.lines.push(SectionHeaderLine { value, kind });
        }
    }

    fn render_lines(&self, width: usize, height: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            lines.push(
                Style::new()
                    .fg(self.title_color)
                    .bold()
                    .render(&fit_visible(
                        &format!("{}{}", " ".repeat(self.indent_for_width(width)), title),
                        width,
                    )),
            );
        }

        let reserved_separator = usize::from(self.show_separator);
        let available = height
            .saturating_sub(lines.len())
            .saturating_sub(reserved_separator);
        for line in self.visible_lines(available) {
            lines.push(self.render_line(line, width));
        }

        if self.show_separator && lines.len() < height {
            lines.push(
                Style::new()
                    .fg(self.separator_color)
                    .render(&"─".repeat(width)),
            );
        }
        lines
    }

    fn render_line(&self, line: &SectionHeaderLine, width: usize) -> String {
        let raw = fit_visible(
            &format!("{}{}", " ".repeat(self.indent_for_width(width)), line.value),
            width,
        );
        match line.kind {
            SectionHeaderLineKind::Metadata => Style::new().fg(self.metadata_color).render(&raw),
            SectionHeaderLineKind::Muted => Style::new().fg(self.muted_color).italic().render(&raw),
        }
    }

    fn line_element(&self, line: &SectionHeaderLine) -> TextElement {
        match line.kind {
            SectionHeaderLineKind::Metadata => {
                TextElement::new(line.value.as_str()).fg(self.metadata_color)
            }
            SectionHeaderLineKind::Muted => TextElement::new(line.value.as_str())
                .fg(self.muted_color)
                .italic(),
        }
    }

    fn visible_lines(&self, available: usize) -> impl Iterator<Item = &SectionHeaderLine> {
        self.lines
            .iter()
            .take(self.max_lines.unwrap_or(usize::MAX))
            .take(available)
    }

    fn indent_for_width(&self, width: usize) -> usize {
        self.indent.min(width).min(MAX_SECTION_HEADER_INDENT)
    }
}

impl Default for SectionHeader {
    fn default() -> Self {
        Self::without_title()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn renders_title_metadata_muted_line_and_separator() {
        let rendered = SectionHeader::new("session view codex · abc")
            .metadata("task build · cwd /Users/roylin/code/a3s")
            .muted("focused session has no events")
            .title_color(Color::Green)
            .view(48, 4);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("session view codex"));
        assert!(plain.contains("task build"));
        assert!(plain.contains("focused session has no events"));
        assert!(plain.contains("──"));
        assert!(rendered.contains("\x1b[3;"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 48, "{line:?}");
        }
    }

    #[test]
    fn respects_height_and_max_lines() {
        let rendered = SectionHeader::new("agent")
            .metadata("pid 42")
            .metadata("cwd /tmp")
            .max_lines(1)
            .view(24, 3);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("agent"));
        assert!(plain.contains("pid 42"));
        assert!(!plain.contains("cwd /tmp"));
        assert_eq!(rendered.lines().count(), 3);
    }

    #[test]
    fn truncates_cjk_metadata_to_width() {
        let rendered = SectionHeader::new("容器")
            .metadata("中文测试内容 with a long explanatory suffix")
            .view(18, 3);

        for line in rendered.lines() {
            assert_eq!(visible_len(line), 18, "{line:?}");
        }
        assert!(strip_ansi(&rendered).contains("中文"));
    }

    #[test]
    fn fill_height_pads_remaining_rows() {
        let rendered = SectionHeader::new("top")
            .show_separator(false)
            .fill_height(true)
            .view(16, 4);

        assert_eq!(rendered.lines().count(), 4);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 16, "{line:?}");
        }
    }

    #[test]
    fn element_with_height_reserves_separator() {
        let el: Element<()> = SectionHeader::new("top")
            .metadata("line one")
            .metadata("line two")
            .element_with_height(3);

        let Element::Box(column) = el else {
            panic!("expected column element");
        };
        let text = column
            .children
            .iter()
            .filter_map(Element::text_content)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(text.contains("top"));
        assert!(text.contains("line one"));
        assert!(text.contains("─"));
        assert!(!text.contains("line two"));
    }
}
