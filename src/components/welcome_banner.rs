use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, visible_len, Color, Style};

/// First-run welcome surface with paired mascot/art rows and startup metadata.
///
/// This extracts the CLI welcome-banner pattern without hard-coding a brand:
/// callers provide the left mascot/illustration rows, right wordmark/art rows,
/// metadata, tips, and optional notice text.
#[derive(Debug, Clone)]
pub struct WelcomeBanner {
    mascot_lines: Vec<String>,
    art_lines: Vec<String>,
    metadata: Vec<String>,
    tips: Vec<String>,
    notice: Option<String>,
    margin: usize,
    gap: usize,
    art_offset: usize,
    fill_height: bool,
    mascot_color: Color,
    art_color: Color,
    metadata_color: Color,
    tip_color: Color,
    notice_color: Color,
}

impl WelcomeBanner {
    pub fn new() -> Self {
        Self {
            mascot_lines: Vec::new(),
            art_lines: Vec::new(),
            metadata: Vec::new(),
            tips: Vec::new(),
            notice: None,
            margin: 2,
            gap: 2,
            art_offset: 0,
            fill_height: false,
            mascot_color: Color::BrightBlack,
            art_color: Color::Cyan,
            metadata_color: Color::BrightBlack,
            tip_color: Color::BrightBlack,
            notice_color: Color::Cyan,
        }
    }

    pub fn mascot_lines(mut self, lines: Vec<impl Into<String>>) -> Self {
        self.mascot_lines = lines.into_iter().map(Into::into).collect();
        self
    }

    pub fn art_lines(mut self, lines: Vec<impl Into<String>>) -> Self {
        self.art_lines = lines.into_iter().map(Into::into).collect();
        self
    }

    pub fn metadata(mut self, line: impl Into<String>) -> Self {
        let line = line.into();
        if !line.is_empty() {
            self.metadata.push(line);
        }
        self
    }

    pub fn tip(mut self, line: impl Into<String>) -> Self {
        let line = line.into();
        if !line.is_empty() {
            self.tips.push(line);
        }
        self
    }

    pub fn notice(mut self, line: impl Into<String>) -> Self {
        self.notice = Some(line.into());
        self
    }

    pub fn margin(mut self, margin: usize) -> Self {
        self.margin = margin;
        self
    }

    pub fn gap(mut self, gap: usize) -> Self {
        self.gap = gap;
        self
    }

    pub fn art_offset(mut self, offset: usize) -> Self {
        self.art_offset = offset;
        self
    }

    pub fn fill_height(mut self, enabled: bool) -> Self {
        self.fill_height = enabled;
        self
    }

    pub fn mascot_color(mut self, color: Color) -> Self {
        self.mascot_color = color;
        self
    }

    pub fn art_color(mut self, color: Color) -> Self {
        self.art_color = color;
        self
    }

    pub fn metadata_color(mut self, color: Color) -> Self {
        self.metadata_color = color;
        self
    }

    pub fn tip_color(mut self, color: Color) -> Self {
        self.tip_color = color;
        self
    }

    pub fn notice_color(mut self, color: Color) -> Self {
        self.notice_color = color;
        self
    }

    pub fn mascot_lines_value(&self) -> &[String] {
        &self.mascot_lines
    }

    pub fn art_lines_value(&self) -> &[String] {
        &self.art_lines
    }

    pub fn metadata_value(&self) -> &[String] {
        &self.metadata
    }

    pub fn tips_value(&self) -> &[String] {
        &self.tips
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
        for row in self.logo_rows() {
            children.push(self.logo_row_element(row));
        }

        if !self.logo_rows().is_empty() && (!self.metadata.is_empty() || !self.tips.is_empty()) {
            children.push(Element::Text(TextElement::new("")));
        }

        for line in &self.metadata {
            children.push(Element::Text(
                TextElement::new(self.indented(line)).fg(self.metadata_color),
            ));
        }
        for line in &self.tips {
            children.push(Element::Text(
                TextElement::new(self.indented(line))
                    .fg(self.tip_color)
                    .italic(),
            ));
        }
        if let Some(notice) = self.notice.as_deref().filter(|notice| !notice.is_empty()) {
            if !self.metadata.is_empty() || !self.tips.is_empty() {
                children.push(Element::Text(TextElement::new("")));
            }
            children.push(Element::Text(
                TextElement::new(self.indented(notice))
                    .fg(self.notice_color)
                    .bold(),
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
        for row in self.logo_rows() {
            lines.push(self.render_logo_row(row));
        }

        if !lines.is_empty() && (!self.metadata.is_empty() || !self.tips.is_empty()) {
            lines.push(String::new());
        }

        for line in &self.metadata {
            lines.push(
                Style::new()
                    .fg(self.metadata_color)
                    .render(&fit_visible(&self.indented(line), width)),
            );
        }
        for line in &self.tips {
            lines.push(
                Style::new()
                    .fg(self.tip_color)
                    .italic()
                    .render(&fit_visible(&self.indented(line), width)),
            );
        }
        if let Some(notice) = self.notice.as_deref().filter(|notice| !notice.is_empty()) {
            if !self.metadata.is_empty() || !self.tips.is_empty() {
                lines.push(String::new());
            }
            lines.push(
                Style::new()
                    .fg(self.notice_color)
                    .bold()
                    .render(&fit_visible(&self.indented(notice), width)),
            );
        }

        lines
    }

    fn render_logo_row(&self, row: LogoRow) -> String {
        let mut line = " ".repeat(self.margin);
        if !row.mascot.is_empty() {
            line.push_str(
                &Style::new()
                    .fg(self.mascot_color)
                    .bold()
                    .render(&row.mascot),
            );
        }
        if !row.art.is_empty() {
            line.push_str(&" ".repeat(self.gap));
            line.push_str(&Style::new().fg(self.art_color).bold().render(&row.art));
        }
        line
    }

    fn logo_row_element<Msg>(&self, row: LogoRow) -> Element<Msg> {
        let mut children = vec![Element::Text(TextElement::new(" ".repeat(self.margin)))];
        if !row.mascot.is_empty() {
            children.push(Element::Text(
                TextElement::new(row.mascot).fg(self.mascot_color).bold(),
            ));
        }
        if !row.art.is_empty() {
            children.push(Element::Text(TextElement::new(" ".repeat(self.gap))));
            children.push(Element::Text(
                TextElement::new(row.art).fg(self.art_color).bold(),
            ));
        }
        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .children(children),
        )
    }

    fn logo_rows(&self) -> Vec<LogoRow> {
        let mascot_width = self
            .mascot_lines
            .iter()
            .map(|line| visible_len(line))
            .max()
            .unwrap_or(0);
        let row_count = self
            .mascot_lines
            .len()
            .max(self.art_offset.saturating_add(self.art_lines.len()));
        let mut rows = Vec::with_capacity(row_count);
        for index in 0..row_count {
            let mascot = self
                .mascot_lines
                .get(index)
                .map(|line| pad_to_width(line, mascot_width))
                .unwrap_or_else(|| " ".repeat(mascot_width));
            let art = index
                .checked_sub(self.art_offset)
                .and_then(|art_index| self.art_lines.get(art_index))
                .cloned()
                .unwrap_or_default();
            rows.push(LogoRow { mascot, art });
        }
        rows
    }

    fn indented(&self, value: &str) -> String {
        format!("{}{}", " ".repeat(self.margin), value)
    }
}

impl Default for WelcomeBanner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct LogoRow {
    mascot: String,
    art: String,
}

fn pad_to_width(value: &str, width: usize) -> String {
    let len = visible_len(value);
    if len >= width {
        value.to_string()
    } else {
        format!("{value}{}", " ".repeat(width - len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    fn sample() -> WelcomeBanner {
        WelcomeBanner::new()
            .mascot_lines(vec!["  .-.  ", " (o o) ", " /|_|\\ "])
            .art_lines(vec!["A3S CODE", "TERMINAL UI"])
            .art_offset(1)
            .metadata("a3s-code v0.5.0 · gpt-5 · /Users/roylin/code/a3s")
            .tip("Type a message · / for commands · Shift+Tab cycles mode")
            .notice("a3s 0.6.0 is available · type /update")
    }

    #[test]
    fn renders_paired_logo_metadata_tips_and_notice() {
        let rendered = sample().view(72, 8);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains(".-."));
        assert!(plain.contains("A3S CODE"));
        assert!(plain.contains("a3s-code v0.5.0"));
        assert!(plain.contains("Type a message"));
        assert!(plain.contains("0.6.0 is available"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 72, "{line:?}");
        }
    }

    #[test]
    fn art_offset_aligns_wordmark_below_mascot_first_row() {
        let plain = strip_ansi(&sample().view(60, 4));
        let rows = plain.lines().collect::<Vec<_>>();

        assert!(!rows[0].contains("A3S CODE"));
        assert!(rows[1].contains("A3S CODE"));
        assert!(rows[2].contains("TERMINAL UI"));
    }

    #[test]
    fn cjk_metadata_fits_requested_width() {
        let rendered = WelcomeBanner::new()
            .art_lines(vec!["A3S"])
            .metadata("模型 OS 网关 · 中文路径 very-long-tail")
            .view(24, 4);

        assert!(strip_ansi(&rendered).contains("模型"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 24, "{line:?}");
        }
    }

    #[test]
    fn fill_height_pads_remaining_rows() {
        let rendered = sample().fill_height(true).view(40, 10);

        assert_eq!(rendered.lines().count(), 10);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 40, "{line:?}");
        }
    }

    #[test]
    fn zero_size_renders_empty_string() {
        assert_eq!(sample().view(0, 4), "");
        assert_eq!(sample().view(40, 0), "");
    }

    #[test]
    fn element_produces_column() {
        let el: Element<()> = sample().element();

        match el {
            Element::Box(column) => {
                assert_eq!(column.style.flex_direction, FlexDirection::Column);
                assert!(!column.children.is_empty());
            }
            _ => panic!("expected Box"),
        }
    }
}
