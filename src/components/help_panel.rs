use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, Color, Style};
use crate::theme::{Theme, ThemeRole};

const MAX_HELP_PANEL_GAP: usize = u16::MAX as usize;
const MAX_HELP_PANEL_INDENT: usize = u16::MAX as usize;
const MAX_HELP_PANEL_KEY_WIDTH: usize = u16::MAX as usize;

/// One key/action row inside a help section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpRow {
    key: String,
    description: String,
}

impl HelpRow {
    pub fn new(key: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

/// A group of help rows under a section title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpSection {
    title: String,
    rows: Vec<HelpRow>,
}

impl HelpSection {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            rows: Vec::new(),
        }
    }

    pub fn row(mut self, key: impl Into<String>, description: impl Into<String>) -> Self {
        self.rows.push(HelpRow::new(key, description));
        self
    }

    pub fn add_row(&mut self, key: impl Into<String>, description: impl Into<String>) {
        self.rows.push(HelpRow::new(key, description));
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn rows(&self) -> &[HelpRow] {
        &self.rows
    }
}

/// A full-width help surface for keyboard shortcuts and command references.
///
/// This component extracts the common terminal pattern used by CLI help screens:
/// a title, grouped section headers, fixed-width key labels, descriptions, and
/// optional footer text. The text renderer is display-width aware and ANSI safe.
#[derive(Debug, Clone)]
pub struct HelpPanel {
    title: Option<String>,
    sections: Vec<HelpSection>,
    footer: Option<String>,
    key_width: usize,
    indent: usize,
    gap: usize,
    fill_height: bool,
    title_color: Color,
    section_color: Color,
    key_color: Color,
    description_color: Color,
    footer_color: Color,
}

impl HelpPanel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            sections: Vec::new(),
            footer: None,
            key_width: 16,
            indent: 4,
            gap: 2,
            fill_height: false,
            title_color: Color::Cyan,
            section_color: Color::Cyan,
            key_color: Color::White,
            description_color: Color::BrightBlack,
            footer_color: Color::BrightBlack,
        }
    }

    pub fn without_title() -> Self {
        Self {
            title: None,
            ..Self::new("")
        }
    }

    pub fn section(mut self, section: HelpSection) -> Self {
        self.sections.push(section);
        self
    }

    pub fn add_section(&mut self, section: HelpSection) {
        self.sections.push(section);
    }

    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    pub fn key_width(mut self, width: usize) -> Self {
        self.key_width = width.min(MAX_HELP_PANEL_KEY_WIDTH);
        self
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent.min(MAX_HELP_PANEL_INDENT);
        self
    }

    pub fn gap(mut self, gap: usize) -> Self {
        self.gap = gap.min(MAX_HELP_PANEL_GAP);
        self
    }

    pub fn fill_height(mut self, enabled: bool) -> Self {
        self.fill_height = enabled;
        self
    }

    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    pub fn section_color(mut self, color: Color) -> Self {
        self.section_color = color;
        self
    }

    pub fn key_color(mut self, color: Color) -> Self {
        self.key_color = color;
        self
    }

    pub fn description_color(mut self, color: Color) -> Self {
        self.description_color = color;
        self
    }

    pub fn footer_color(mut self, color: Color) -> Self {
        self.footer_color = color;
        self
    }

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.title_color = theme.color(ThemeRole::Primary);
        self.section_color = theme.color(ThemeRole::Primary);
        self.key_color = theme.color(ThemeRole::Foreground);
        self.description_color = theme.color(ThemeRole::Muted);
        self.footer_color = theme.color(ThemeRole::Muted);
        self
    }

    pub fn sections(&self) -> &[HelpSection] {
        &self.sections
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        let mut lines = self.render_lines(width);
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
        self.push_section_elements(&mut children, usize::MAX);
        self.push_footer_element(&mut children, usize::MAX);

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
        self.push_section_elements(&mut children, height);
        self.push_footer_element(&mut children, height);
        children.truncate(height);

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn push_section_elements<Msg>(&self, children: &mut Vec<Element<Msg>>, height: usize) {
        for section in &self.sections {
            if children.len() >= height {
                break;
            }
            children.push(Element::Text(
                TextElement::new(section.title.as_str())
                    .fg(self.section_color)
                    .bold(),
            ));
            for row in &section.rows {
                if children.len() >= height {
                    break;
                }
                children.push(Element::Box(
                    BoxElement::new()
                        .direction(FlexDirection::Row)
                        .child(Element::Text(
                            TextElement::new(row.key.as_str()).fg(self.key_color).bold(),
                        ))
                        .child(Element::Text(TextElement::new(
                            " ".repeat(self.gap_for_element()),
                        )))
                        .child(Element::Text(
                            TextElement::new(row.description.as_str()).fg(self.description_color),
                        )),
                ));
            }
        }
    }

    fn push_footer_element<Msg>(&self, children: &mut Vec<Element<Msg>>, height: usize) {
        if children.len() >= height {
            return;
        }
        if let Some(footer) = &self.footer {
            children.push(Element::Text(
                TextElement::new(footer.as_str()).fg(self.footer_color),
            ));
        }
    }

    fn render_lines(&self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();

        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            lines.push(self.render_heading(title, self.title_color));
            if !self.sections.is_empty() || self.footer.is_some() {
                lines.push(String::new());
            }
        }

        for (index, section) in self.sections.iter().enumerate() {
            if index > 0 {
                lines.push(String::new());
            }
            lines.push(self.render_heading(&section.title, self.section_color));
            for row in &section.rows {
                lines.push(self.render_row(row, width));
            }
        }

        if let Some(footer) = &self.footer {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            lines.push(Style::new().fg(self.footer_color).render(footer));
        }

        lines
    }

    fn render_heading(&self, title: &str, color: Color) -> String {
        Style::new().fg(color).bold().render(title)
    }

    fn render_row(&self, row: &HelpRow, width: usize) -> String {
        let (indent_width, key_width, gap_width) = self.spacing_for_width(width);
        let indent = " ".repeat(indent_width);
        let key = fit_visible(&row.key, key_width);
        let gap = " ".repeat(gap_width);
        let used = [indent_width, key_width, gap_width]
            .into_iter()
            .fold(0usize, usize::saturating_add);
        let description_width = width.saturating_sub(used);
        let description = truncate_visible(&row.description, description_width);

        format!(
            "{indent}{}{}{}",
            Style::new().fg(self.key_color).bold().render(&key),
            gap,
            Style::new().fg(self.description_color).render(&description)
        )
    }

    fn spacing_for_width(&self, width: usize) -> (usize, usize, usize) {
        let indent = self.indent.min(width).min(MAX_HELP_PANEL_INDENT);
        let remaining = width.saturating_sub(indent);
        let key_width = self.key_width.min(remaining).min(MAX_HELP_PANEL_KEY_WIDTH);
        let remaining = remaining.saturating_sub(key_width);
        let gap = self.gap.min(remaining).min(MAX_HELP_PANEL_GAP);
        (indent, key_width, gap)
    }

    fn gap_for_element(&self) -> usize {
        self.gap.min(MAX_HELP_PANEL_GAP)
    }
}

impl Default for HelpPanel {
    fn default() -> Self {
        Self::without_title()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    fn sample_panel() -> HelpPanel {
        HelpPanel::new("A3S CODE - help")
            .section(
                HelpSection::new("Slash commands")
                    .row("/model", "pick the model")
                    .row("/help", "this panel"),
            )
            .section(
                HelpSection::new("Keys")
                    .row("Enter", "send")
                    .row("Esc", "close panel"),
            )
            .footer("Resume with: a3s code resume <id>")
    }

    #[test]
    fn renders_title_sections_rows_and_footer() {
        let plain = strip_ansi(&sample_panel().view(60, 20));

        assert!(plain.contains("A3S CODE - help"));
        assert!(plain.contains("Slash commands"));
        assert!(plain.contains("/model"));
        assert!(plain.contains("pick the model"));
        assert!(plain.contains("Resume with:"));
    }

    #[test]
    fn pads_rows_to_requested_width() {
        let rendered = sample_panel().fill_height(true).view(48, 12);

        assert_eq!(rendered.lines().count(), 12);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 48, "{line:?}");
        }
    }

    #[test]
    fn truncates_long_keys_and_descriptions() {
        let panel = HelpPanel::without_title()
            .key_width(8)
            .section(HelpSection::new("Keys").row(
                "very-long-key",
                "中文测试内容 with an especially long explanation",
            ));
        let rendered = panel.view(24, 4);

        for line in rendered.lines() {
            assert!(visible_len(line) <= 24, "{line:?}");
        }
        let plain = strip_ansi(&rendered);
        assert!(plain.contains("very-lo…"));
    }

    #[test]
    fn height_limits_rows_without_filling_by_default() {
        let rendered = sample_panel().view(60, 3);

        assert_eq!(rendered.lines().count(), 3);
    }

    #[test]
    fn element_with_height_limits_rows() {
        let Element::Box(column) = sample_panel().element_with_height::<()>(3) else {
            panic!("expected column element");
        };
        let text = column
            .children
            .iter()
            .flat_map(|child| match child {
                Element::Text(text) => vec![text.content.as_str()],
                Element::Box(row) => row
                    .children
                    .iter()
                    .filter_map(Element::text_content)
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(column.children.len(), 3);
        assert!(text.contains("A3S CODE - help"));
        assert!(text.contains("Slash commands"));
        assert!(text.contains("/model"));
        assert!(!text.contains("/help"));
        assert!(!text.contains("Resume with:"));
    }

    #[test]
    fn oversized_spacing_is_clamped_to_render_width() {
        let panel = HelpPanel::without_title()
            .indent(usize::MAX)
            .gap(usize::MAX)
            .key_width(usize::MAX)
            .section(HelpSection::new("Keys").row("Enter", "send"));
        let rendered = panel.view(8, 4);
        let raw_row = panel.render_row(panel.sections[0].rows.first().unwrap(), 8);

        assert_eq!(panel.indent, MAX_HELP_PANEL_INDENT);
        assert_eq!(panel.gap, MAX_HELP_PANEL_GAP);
        assert_eq!(panel.key_width, MAX_HELP_PANEL_KEY_WIDTH);
        assert_eq!(visible_len(&raw_row), 8);
        assert!(rendered.lines().all(|line| visible_len(line) == 8));

        let Element::Box(column) = panel.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Box(row) = &column.children[1] else {
            panic!("expected row element");
        };
        let Element::Text(gap) = &row.children[1] else {
            panic!("expected gap text");
        };
        assert_eq!(gap.content.len(), MAX_HELP_PANEL_GAP);
    }

    #[test]
    fn element_produces_column() {
        let el: Element<()> = sample_panel().element();

        match el {
            Element::Box(column) => {
                assert_eq!(column.style.flex_direction, FlexDirection::Column);
                assert!(!column.children.is_empty());
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let panel = HelpPanel::new("Help").with_theme(&theme);

        assert_eq!(panel.title_color, theme.color(ThemeRole::Primary));
        assert_eq!(panel.section_color, theme.color(ThemeRole::Primary));
        assert_eq!(panel.key_color, theme.color(ThemeRole::Foreground));
        assert_eq!(panel.description_color, theme.color(ThemeRole::Muted));
        assert_eq!(panel.footer_color, theme.color(ThemeRole::Muted));
    }
}
