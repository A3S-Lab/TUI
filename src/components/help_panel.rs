use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, truncate_visible, Color, Style};

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
        self.key_width = width;
        self
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn gap(mut self, gap: usize) -> Self {
        self.gap = gap;
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

        for section in &self.sections {
            children.push(Element::Text(
                TextElement::new(section.title.as_str())
                    .fg(self.section_color)
                    .bold(),
            ));
            for row in &section.rows {
                children.push(Element::Box(
                    BoxElement::new()
                        .direction(FlexDirection::Row)
                        .child(Element::Text(
                            TextElement::new(row.key.as_str()).fg(self.key_color).bold(),
                        ))
                        .child(Element::Text(TextElement::new(" ".repeat(self.gap))))
                        .child(Element::Text(
                            TextElement::new(row.description.as_str()).fg(self.description_color),
                        )),
                ));
            }
        }

        if let Some(footer) = &self.footer {
            children.push(Element::Text(
                TextElement::new(footer.as_str()).fg(self.footer_color),
            ));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
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
        let indent = " ".repeat(self.indent);
        let key = fit_visible(&row.key, self.key_width);
        let gap = " ".repeat(self.gap);
        let used = self.indent + self.key_width + self.gap;
        let description_width = width.saturating_sub(used);
        let description = truncate_visible(&row.description, description_width);

        format!(
            "{indent}{}{}{}",
            Style::new().fg(self.key_color).bold().render(&key),
            gap,
            Style::new().fg(self.description_color).render(&description)
        )
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
}
