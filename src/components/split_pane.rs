use crate::element::{BoxElement, Dimension, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, visible_len, Color, Style};

/// A two-column terminal panel for file explorers, diffs, timelines, and details.
///
/// `SplitPane` extracts the common full-screen pattern used by IDE, git, memory,
/// and detail views: optional heading rows, a left list/tree, a right detail
/// pane, a visible separator, and an optional footer. The line renderer is
/// display-width aware and preserves ANSI styling while truncating each pane.
#[derive(Debug, Clone)]
pub struct SplitPane {
    title: Option<String>,
    subtitle: Option<String>,
    left_title: Option<String>,
    right_title: Option<String>,
    footer: Option<String>,
    left: Vec<String>,
    right: Vec<String>,
    left_width: Option<usize>,
    left_ratio: f32,
    separator: String,
    fill_height: bool,
    title_color: Color,
    subtitle_color: Color,
    pane_title_color: Color,
    separator_color: Color,
    footer_color: Color,
}

impl SplitPane {
    pub fn new(left: Vec<impl Into<String>>, right: Vec<impl Into<String>>) -> Self {
        Self {
            title: None,
            subtitle: None,
            left_title: None,
            right_title: None,
            footer: None,
            left: left.into_iter().map(Into::into).collect(),
            right: right.into_iter().map(Into::into).collect(),
            left_width: None,
            left_ratio: 0.36,
            separator: " │ ".to_string(),
            fill_height: false,
            title_color: Color::Cyan,
            subtitle_color: Color::BrightBlack,
            pane_title_color: Color::BrightBlack,
            separator_color: Color::BrightBlack,
            footer_color: Color::BrightBlack,
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::<String>::new(), Vec::<String>::new())
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn pane_titles(mut self, left: impl Into<String>, right: impl Into<String>) -> Self {
        self.left_title = Some(left.into());
        self.right_title = Some(right.into());
        self
    }

    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    pub fn left_lines(mut self, lines: Vec<impl Into<String>>) -> Self {
        self.left = lines.into_iter().map(Into::into).collect();
        self
    }

    pub fn right_lines(mut self, lines: Vec<impl Into<String>>) -> Self {
        self.right = lines.into_iter().map(Into::into).collect();
        self
    }

    pub fn left_width(mut self, width: usize) -> Self {
        self.left_width = Some(width);
        self
    }

    pub fn left_ratio(mut self, ratio: f32) -> Self {
        self.left_ratio = ratio.clamp(0.1, 0.9);
        self
    }

    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
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

    pub fn subtitle_color(mut self, color: Color) -> Self {
        self.subtitle_color = color;
        self
    }

    pub fn pane_title_color(mut self, color: Color) -> Self {
        self.pane_title_color = color;
        self
    }

    pub fn separator_color(mut self, color: Color) -> Self {
        self.separator_color = color;
        self
    }

    pub fn footer_color(mut self, color: Color) -> Self {
        self.footer_color = color;
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
        if let Some(subtitle) = self
            .subtitle
            .as_deref()
            .filter(|subtitle| !subtitle.is_empty())
        {
            children.push(Element::Text(
                TextElement::new(subtitle).fg(self.subtitle_color),
            ));
        }

        let mut left = BoxElement::new()
            .direction(FlexDirection::Column)
            .width(Dimension::Percent(self.left_ratio * 100.0));
        if let Some(title) = self.left_title.as_deref().filter(|title| !title.is_empty()) {
            left = left.child(Element::Text(
                TextElement::new(title).fg(self.pane_title_color).bold(),
            ));
        }
        for line in &self.left {
            left = left.child(Element::Text(TextElement::new(line.as_str())));
        }

        let mut right = BoxElement::new()
            .direction(FlexDirection::Column)
            .flex_grow(1.0);
        if let Some(title) = self
            .right_title
            .as_deref()
            .filter(|title| !title.is_empty())
        {
            right = right.child(Element::Text(
                TextElement::new(title).fg(self.pane_title_color).bold(),
            ));
        }
        for line in &self.right {
            right = right.child(Element::Text(TextElement::new(line.as_str())));
        }

        children.push(Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .child(Element::Box(left))
                .child(Element::Text(
                    TextElement::new(self.separator.as_str()).fg(self.separator_color),
                ))
                .child(Element::Box(right)),
        ));

        if let Some(footer) = self.footer.as_deref().filter(|footer| !footer.is_empty()) {
            children.push(Element::Text(
                TextElement::new(footer).fg(self.footer_color),
            ));
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_lines(&self, width: usize, height: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            lines.push(
                Style::new()
                    .fg(self.title_color)
                    .bold()
                    .render(&fit_visible(title, width)),
            );
        }
        if let Some(subtitle) = self
            .subtitle
            .as_deref()
            .filter(|subtitle| !subtitle.is_empty())
        {
            lines.push(
                Style::new()
                    .fg(self.subtitle_color)
                    .render(&fit_visible(subtitle, width)),
            );
        }

        let has_pane_titles = self
            .left_title
            .as_ref()
            .is_some_and(|title| !title.is_empty())
            || self
                .right_title
                .as_ref()
                .is_some_and(|title| !title.is_empty());
        let footer_rows = usize::from(self.footer.as_ref().is_some_and(|f| !f.is_empty()));
        let reserved = lines.len() + usize::from(has_pane_titles) + footer_rows;
        let body_rows = height.saturating_sub(reserved);
        let (left_width, right_width, separator) = self.column_widths(width);

        if has_pane_titles {
            lines.push(self.render_joined(
                self.left_title.as_deref().unwrap_or_default(),
                self.right_title.as_deref().unwrap_or_default(),
                left_width,
                right_width,
                &separator,
                Some(self.pane_title_color),
            ));
        }

        for index in 0..body_rows {
            let left = self.left.get(index).map(String::as_str).unwrap_or_default();
            let right = self
                .right
                .get(index)
                .map(String::as_str)
                .unwrap_or_default();
            if left.is_empty() && right.is_empty() && !self.fill_height {
                break;
            }
            lines.push(self.render_joined(left, right, left_width, right_width, &separator, None));
        }

        if let Some(footer) = self.footer.as_deref().filter(|footer| !footer.is_empty()) {
            lines.push(
                Style::new()
                    .fg(self.footer_color)
                    .render(&fit_visible(footer, width)),
            );
        }

        lines
    }

    fn render_joined(
        &self,
        left: &str,
        right: &str,
        left_width: usize,
        right_width: usize,
        separator: &str,
        color: Option<Color>,
    ) -> String {
        let left = fit_visible(left, left_width);
        let right = fit_visible(right, right_width);
        let row = format!("{left}{separator}{right}");
        match color {
            Some(color) => Style::new().fg(color).bold().render(&row),
            None => row,
        }
    }

    fn column_widths(&self, width: usize) -> (usize, usize, String) {
        let separator_width = visible_len(&self.separator);
        if width <= separator_width + 2 {
            return (width, 0, String::new());
        }

        let available = width.saturating_sub(separator_width);
        let min_side = available.min(6);
        let max_left = available.saturating_sub(min_side);
        let proposed = self
            .left_width
            .unwrap_or_else(|| (available as f32 * self.left_ratio).round() as usize);
        let left = proposed.clamp(min_side, max_left.max(min_side));
        let right = available.saturating_sub(left);
        let separator = Style::new()
            .fg(self.separator_color)
            .render(&self.separator);

        (left, right, separator)
    }
}

impl Default for SplitPane {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn renders_title_pane_titles_body_and_footer() {
        let pane = SplitPane::new(
            vec!["src/main.rs", "src/lib.rs"],
            vec!["diff --git", "+ added"],
        )
        .title("Git")
        .subtitle("status / diff")
        .pane_titles("Files", "Diff")
        .footer("Enter stage · Esc close")
        .left_width(14)
        .fill_height(true);

        let rendered = pane.view(48, 7);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("Git"));
        assert!(plain.contains("Files"));
        assert!(plain.contains("Diff"));
        assert!(plain.contains("src/main.rs"));
        assert!(plain.contains("Enter stage"));
        assert_eq!(rendered.lines().count(), 7);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 48, "{line:?}");
        }
    }

    #[test]
    fn truncates_wide_text_per_pane() {
        let pane = SplitPane::new(
            vec!["中文测试内容文件名.rs"],
            vec!["right pane has a very long line with 中文 suffix"],
        )
        .left_width(10);

        let rendered = pane.view(32, 2);

        for line in rendered.lines() {
            assert_eq!(visible_len(line), 32, "{line:?}");
        }
        assert!(strip_ansi(&rendered).contains('…'));
    }

    #[test]
    fn ratio_controls_default_left_width() {
        let pane = SplitPane::new(vec!["left"], vec!["right"]).left_ratio(0.5);
        let rendered = strip_ansi(&pane.view(24, 1));
        let row = rendered.lines().next().unwrap();

        assert!(row.starts_with("left"));
        assert!(row.contains("│"));
        assert_eq!(visible_len(row), 24);
    }

    #[test]
    fn element_produces_column_with_row_body() {
        let el: Element<()> = SplitPane::new(vec!["left"], vec!["right"])
            .title("Title")
            .pane_titles("A", "B")
            .footer("footer")
            .element();

        match el {
            Element::Box(column) => {
                assert_eq!(column.style.flex_direction, FlexDirection::Column);
                assert_eq!(column.children.len(), 3);
                match &column.children[1] {
                    Element::Box(row) => assert_eq!(row.style.flex_direction, FlexDirection::Row),
                    _ => panic!("expected row body"),
                }
            }
            _ => panic!("expected Box"),
        }
    }
}
