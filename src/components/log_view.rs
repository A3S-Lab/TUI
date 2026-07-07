use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{fit_visible, split_nonempty_lines_preserving_trailing_blank, Color, Style};
use crate::theme::{Theme, ThemeRole};

/// Current display state for a [`LogView`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogViewState {
    #[default]
    Ready,
    Loading,
    Refreshing,
}

impl LogViewState {
    pub fn label(self) -> &'static str {
        match self {
            LogViewState::Ready => "ready",
            LogViewState::Loading => "loading",
            LogViewState::Refreshing => "refreshing",
        }
    }
}

/// A scrollable log/output panel with a title, status metadata, and empty states.
///
/// `LogView` extracts the common terminal pattern used by container logs, tool
/// output previews, and background-task transcripts: a bold heading, muted
/// metadata, a separator, and a fixed-height body that is display-width safe.
#[derive(Debug, Clone)]
pub struct LogView {
    title: Option<String>,
    metadata: Vec<String>,
    lines: Vec<String>,
    scroll: usize,
    state: LogViewState,
    empty_text: String,
    loading_text: String,
    footer: Option<String>,
    show_separator: bool,
    fill_height: bool,
    title_color: Color,
    metadata_color: Color,
    text_color: Color,
    muted_color: Color,
    separator_color: Color,
}

impl LogView {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            metadata: Vec::new(),
            lines: Vec::new(),
            scroll: 0,
            state: LogViewState::Ready,
            empty_text: "no log lines".to_string(),
            loading_text: "loading logs...".to_string(),
            footer: None,
            show_separator: true,
            fill_height: false,
            title_color: Color::Cyan,
            metadata_color: Color::BrightBlack,
            text_color: Color::White,
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
        let value = value.into();
        if !value.is_empty() {
            self.metadata.push(value);
        }
        self
    }

    pub fn lines(mut self, lines: Vec<impl Into<String>>) -> Self {
        self.lines = lines.into_iter().map(Into::into).collect();
        self.clamp_scroll();
        self
    }

    pub fn text(mut self, text: impl AsRef<str>) -> Self {
        self.lines = split_nonempty_lines_preserving_trailing_blank(text.as_ref())
            .into_iter()
            .map(str::to_string)
            .collect();
        self.clamp_scroll();
        self
    }

    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.lines.push(line.into());
        self
    }

    pub fn add_line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    pub fn scroll(mut self, scroll: usize) -> Self {
        self.scroll = scroll;
        self.clamp_scroll();
        self
    }

    pub fn state(mut self, state: LogViewState) -> Self {
        self.state = state;
        self
    }

    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty_text = text.into();
        self
    }

    pub fn loading_text(mut self, text: impl Into<String>) -> Self {
        self.loading_text = text.into();
        self
    }

    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
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

    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    pub fn metadata_color(mut self, color: Color) -> Self {
        self.metadata_color = color;
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
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

    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.title_color = theme.color(ThemeRole::Primary);
        self.metadata_color = theme.color(ThemeRole::Muted);
        self.text_color = theme.color(ThemeRole::Foreground);
        self.muted_color = theme.color(ThemeRole::Muted);
        self.separator_color = theme.color(ThemeRole::Border);
        self
    }

    pub fn lines_value(&self) -> &[String] {
        &self.lines
    }

    pub fn scroll_value(&self) -> usize {
        self.normalized_scroll()
    }

    pub fn state_value(&self) -> LogViewState {
        self.state
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
        if let Some(title) = self.title_line() {
            children.push(Element::Text(
                TextElement::new(title).fg(self.title_color).bold(),
            ));
        }
        if self.show_separator && self.has_title() {
            children.push(Element::Text(
                TextElement::new("─").fg(self.separator_color),
            ));
        }

        match self.state {
            LogViewState::Loading => {
                children.push(Element::Text(
                    TextElement::new(self.loading_text.as_str())
                        .fg(self.muted_color)
                        .italic(),
                ));
            }
            LogViewState::Ready | LogViewState::Refreshing => {
                if self.lines.is_empty() {
                    children.push(Element::Text(
                        TextElement::new(self.empty_text.as_str())
                            .fg(self.muted_color)
                            .italic(),
                    ));
                } else {
                    let scroll = self.normalized_scroll();
                    for line in self.lines.iter().skip(scroll) {
                        children.push(Element::Text(
                            TextElement::new(clean_log_line(line)).fg(self.text_color),
                        ));
                    }
                }
            }
        }

        if let Some(footer) = self.footer.as_deref().filter(|footer| !footer.is_empty()) {
            children.push(Element::Text(
                TextElement::new(footer).fg(self.metadata_color),
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

        if let Some(title) = self.title_line() {
            children.push(Element::Text(
                TextElement::new(title).fg(self.title_color).bold(),
            ));
        }
        if self.show_separator && self.has_title() && children.len() < height {
            children.push(Element::Text(
                TextElement::new("─").fg(self.separator_color),
            ));
        }

        let footer_rows = usize::from(self.footer.as_ref().is_some_and(|f| !f.is_empty()));
        let body_height = height.saturating_sub(children.len() + footer_rows);
        match self.state {
            LogViewState::Loading if body_height > 0 => {
                children.push(Element::Text(
                    TextElement::new(self.loading_text.as_str())
                        .fg(self.muted_color)
                        .italic(),
                ));
            }
            LogViewState::Ready | LogViewState::Refreshing
                if self.lines.is_empty() && body_height > 0 =>
            {
                children.push(Element::Text(
                    TextElement::new(self.empty_text.as_str())
                        .fg(self.muted_color)
                        .italic(),
                ));
            }
            LogViewState::Ready | LogViewState::Refreshing => {
                let scroll = self.normalized_scroll_for_body_height(body_height);
                for line in self.lines.iter().skip(scroll).take(body_height) {
                    children.push(Element::Text(
                        TextElement::new(clean_log_line(line)).fg(self.text_color),
                    ));
                }
            }
            LogViewState::Loading => {}
        }

        if let Some(footer) = self.footer.as_deref().filter(|footer| !footer.is_empty()) {
            children.push(Element::Text(
                TextElement::new(footer).fg(self.metadata_color),
            ));
        }
        children.truncate(height);

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn render_lines(&self, width: usize, height: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(title) = self.title_line() {
            lines.push(
                Style::new()
                    .fg(self.title_color)
                    .bold()
                    .render(&fit_visible(&title, width)),
            );
        }
        if self.show_separator && self.has_title() && lines.len() < height {
            lines.push(
                Style::new()
                    .fg(self.separator_color)
                    .render(&"─".repeat(width)),
            );
        }

        let footer_rows = usize::from(self.footer.as_ref().is_some_and(|f| !f.is_empty()));
        let body_height = height.saturating_sub(lines.len() + footer_rows);
        match self.state {
            LogViewState::Loading if body_height > 0 => {
                lines.push(self.render_muted(&self.loading_text, width));
            }
            LogViewState::Ready | LogViewState::Refreshing
                if self.lines.is_empty() && body_height > 0 =>
            {
                lines.push(self.render_muted(&self.empty_text, width));
            }
            LogViewState::Ready | LogViewState::Refreshing => {
                let scroll = self.normalized_scroll_for_body_height(body_height);
                for line in self.lines.iter().skip(scroll).take(body_height) {
                    let raw = fit_visible(clean_log_line(line), width);
                    lines.push(Style::new().fg(self.text_color).render(&raw));
                }
            }
            LogViewState::Loading => {}
        }

        if let Some(footer) = self.footer.as_deref().filter(|footer| !footer.is_empty()) {
            lines.push(
                Style::new()
                    .fg(self.metadata_color)
                    .render(&fit_visible(footer, width)),
            );
        }

        lines
    }

    fn title_line(&self) -> Option<String> {
        let title = self.title.as_deref().filter(|title| !title.is_empty())?;
        let mut segments = Vec::with_capacity(self.metadata.len() + 2);
        segments.push(title.to_string());
        if self.state != LogViewState::Ready {
            segments.push(self.state.label().to_string());
        }
        segments.extend(
            self.metadata
                .iter()
                .filter(|value| !value.is_empty())
                .cloned(),
        );
        Some(format!(" {}", segments.join(" · ")))
    }

    fn has_title(&self) -> bool {
        self.title.as_ref().is_some_and(|title| !title.is_empty())
    }

    fn render_muted(&self, text: &str, width: usize) -> String {
        Style::new()
            .fg(self.muted_color)
            .italic()
            .render(&fit_visible(&format!(" {text}"), width))
    }

    fn normalized_scroll(&self) -> usize {
        self.scroll.min(self.lines.len().saturating_sub(1))
    }

    fn normalized_scroll_for_body_height(&self, body_height: usize) -> usize {
        if body_height == 0 || self.lines.is_empty() {
            return 0;
        }

        self.scroll.min(
            self.lines
                .len()
                .saturating_sub(body_height.min(self.lines.len())),
        )
    }

    fn clamp_scroll(&mut self) {
        self.scroll = self.normalized_scroll();
    }
}

impl Default for LogView {
    fn default() -> Self {
        Self::without_title()
    }
}

fn clean_log_line(line: &str) -> &str {
    line.trim_end_matches('\r')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    fn sample() -> LogView {
        LogView::new("logs app")
            .metadata("tail 200")
            .metadata("follow:on")
            .lines(vec!["one", "two", "three"])
    }

    #[test]
    fn renders_title_metadata_separator_and_body() {
        let rendered = sample().view(36, 6);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("logs app"));
        assert!(plain.contains("tail 200"));
        assert!(plain.contains("follow:on"));
        assert!(plain.contains("one"));
        assert!(plain.contains("three"));
        assert!(plain.contains("──"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 36, "{line:?}");
        }
    }

    #[test]
    fn loading_state_replaces_body() {
        let rendered = sample()
            .state(LogViewState::Loading)
            .loading_text("loading container logs...")
            .view(40, 5);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("loading"));
        assert!(plain.contains("loading container logs"));
        assert!(!plain.contains("one"));
    }

    #[test]
    fn empty_state_is_muted_and_italic() {
        let rendered = LogView::new("logs")
            .empty_text("no logs returned")
            .view(32, 4);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("no logs returned"));
        assert!(rendered.contains("\x1b[3;"));
    }

    #[test]
    fn scrolls_body_and_limits_rows() {
        let rendered = sample().scroll(1).view(24, 4);
        let plain = strip_ansi(&rendered);

        assert!(!plain.contains("one"));
        assert!(plain.contains("two"));
        assert_eq!(rendered.lines().count(), 4);
    }

    #[test]
    fn normalizes_stale_scroll_when_rendering() {
        let mut log = sample();
        log.scroll = usize::MAX;

        assert_eq!(log.scroll_value(), 2);

        let rendered = log.view(24, 4);
        let plain = strip_ansi(&rendered);

        assert!(!plain.contains("one"));
        assert!(plain.contains("two"));
        assert!(plain.contains("three"));
    }

    #[test]
    fn text_preserves_trailing_blank_log_line() {
        let log = LogView::without_title().text("one\n");

        assert_eq!(log.lines_value(), ["one", ""]);

        let plain = strip_ansi(&log.view(8, 2));
        let rows = plain.split('\n').collect::<Vec<_>>();
        assert_eq!(rows, vec!["one     ", "        "]);
    }

    #[test]
    fn empty_text_keeps_log_empty() {
        let log = LogView::without_title().text("");

        assert!(log.lines_value().is_empty());
    }

    #[test]
    fn truncates_cjk_and_fills_height() {
        let rendered = LogView::new("logs")
            .line("中文测试内容 with a long suffix")
            .fill_height(true)
            .view(18, 5);

        assert_eq!(rendered.lines().count(), 5);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 18, "{line:?}");
        }
        assert!(strip_ansi(&rendered).contains("中文"));
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

    #[test]
    fn element_with_height_limits_scrolled_body_rows() {
        let el: Element<()> = LogView::without_title()
            .lines(vec!["one", "two", "three"])
            .scroll(1)
            .element_with_height(1);

        let Element::Box(column) = el else {
            panic!("expected column element");
        };
        let text = column
            .children
            .iter()
            .filter_map(Element::text_content)
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(column.children.len(), 1);
        assert_eq!(text, "two");
    }

    #[test]
    fn element_with_height_keeps_footer_budget() {
        let el: Element<()> = LogView::without_title()
            .lines(vec!["one", "two", "three"])
            .scroll(1)
            .footer("tail")
            .element_with_height(2);

        let Element::Box(column) = el else {
            panic!("expected column element");
        };
        let text = column
            .children
            .iter()
            .filter_map(Element::text_content)
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(column.children.len(), 2);
        assert!(text.contains("two"));
        assert!(text.contains("tail"));
        assert!(!text.contains("one"));
        assert!(!text.contains("three"));
    }

    #[test]
    fn empty_title_does_not_render_header_separator() {
        let rendered = LogView::new("").line("body").view(12, 3);
        let plain = strip_ansi(&rendered);

        assert_eq!(plain.lines().next().unwrap().trim(), "body");
        assert!(!plain.contains('─'));
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let view = LogView::new("logs").with_theme(&theme);

        assert_eq!(view.title_color, theme.color(ThemeRole::Primary));
        assert_eq!(view.metadata_color, theme.color(ThemeRole::Muted));
        assert_eq!(view.text_color, theme.color(ThemeRole::Foreground));
        assert_eq!(view.muted_color, theme.color(ThemeRole::Muted));
        assert_eq!(view.separator_color, theme.color(ThemeRole::Border));
    }
}
