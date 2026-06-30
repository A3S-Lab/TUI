use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::style::{fit_visible, strip_ansi, truncate_visible, visible_len, Color, Style};
use crossterm::event::KeyCode;

/// One selectable row in a [`PreviewPanel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewItem {
    label: String,
    description: Option<String>,
    color: Option<Color>,
    disabled: bool,
}

impl PreviewItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            color: None,
            disabled: false,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn description_value(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn color_value(&self) -> Option<Color> {
        self.color
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// Message returned by [`PreviewPanel`] input handlers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewPanelMsg {
    Selected(usize),
    Cancelled,
}

/// Selectable item list with a live preview section.
///
/// This extracts the overlay shape used by theme pickers and similar terminal
/// palettes: a compact selected list, a divider label, and fixed preview rows
/// that may already contain ANSI styling.
#[derive(Debug, Clone)]
pub struct PreviewPanel {
    title: Option<String>,
    subtitle: Option<String>,
    items: Vec<PreviewItem>,
    selected: usize,
    scroll: usize,
    max_items: Option<usize>,
    preview_title: Option<String>,
    preview_lines: Vec<String>,
    footer: Option<String>,
    fill_height: bool,
    y_offset: u16,
    indent: usize,
    marker: String,
    title_color: Color,
    subtitle_color: Color,
    text_color: Color,
    muted_color: Color,
    selected_fg: Color,
    selected_bg: Color,
    disabled_color: Color,
    preview_color: Color,
    divider_color: Color,
}

impl PreviewPanel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            subtitle: None,
            items: Vec::new(),
            selected: 0,
            scroll: 0,
            max_items: None,
            preview_title: Some("preview".to_string()),
            preview_lines: Vec::new(),
            footer: None,
            fill_height: false,
            y_offset: 0,
            indent: 2,
            marker: "▸".to_string(),
            title_color: Color::Cyan,
            subtitle_color: Color::BrightBlack,
            text_color: Color::White,
            muted_color: Color::BrightBlack,
            selected_fg: Color::BrightWhite,
            selected_bg: Color::Cyan,
            disabled_color: Color::BrightBlack,
            preview_color: Color::White,
            divider_color: Color::BrightBlack,
        }
    }

    pub fn without_title() -> Self {
        Self {
            title: None,
            ..Self::new("")
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn item(mut self, item: PreviewItem) -> Self {
        self.items.push(item);
        self.clamp_selection();
        self
    }

    pub fn items(mut self, items: Vec<PreviewItem>) -> Self {
        self.items = items;
        self.clamp_selection();
        self
    }

    pub fn add_item(&mut self, item: PreviewItem) {
        self.items.push(item);
        self.clamp_selection();
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self.clamp_selection();
        self
    }

    pub fn scroll(mut self, scroll: usize) -> Self {
        self.scroll = scroll;
        self
    }

    pub fn max_items(mut self, max_items: usize) -> Self {
        self.max_items = Some(max_items.max(1));
        self
    }

    pub fn preview_title(mut self, title: impl Into<String>) -> Self {
        self.preview_title = Some(title.into());
        self
    }

    pub fn without_preview_title(mut self) -> Self {
        self.preview_title = None;
        self
    }

    pub fn preview_lines(mut self, lines: Vec<impl Into<String>>) -> Self {
        self.preview_lines = lines.into_iter().map(Into::into).collect();
        self
    }

    pub fn preview_text(mut self, text: impl AsRef<str>) -> Self {
        self.preview_lines = text.as_ref().lines().map(str::to_string).collect();
        self
    }

    pub fn preview_line(mut self, line: impl Into<String>) -> Self {
        self.preview_lines.push(line.into());
        self
    }

    pub fn add_preview_line(&mut self, line: impl Into<String>) {
        self.preview_lines.push(line.into());
    }

    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    pub fn fill_height(mut self, enabled: bool) -> Self {
        self.fill_height = enabled;
        self
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn marker(mut self, marker: impl Into<String>) -> Self {
        let marker = marker.into();
        if !marker.is_empty() {
            self.marker = marker;
        }
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

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    pub fn muted_color(mut self, color: Color) -> Self {
        self.muted_color = color;
        self
    }

    pub fn selected_colors(mut self, fg: Color, bg: Color) -> Self {
        self.selected_fg = fg;
        self.selected_bg = bg;
        self
    }

    pub fn disabled_color(mut self, color: Color) -> Self {
        self.disabled_color = color;
        self
    }

    pub fn preview_color(mut self, color: Color) -> Self {
        self.preview_color = color;
        self
    }

    pub fn divider_color(mut self, color: Color) -> Self {
        self.divider_color = color;
        self
    }

    pub fn set_y_offset(&mut self, y: u16) {
        self.y_offset = y;
    }

    pub fn items_value(&self) -> &[PreviewItem] {
        &self.items
    }

    pub fn preview_lines_value(&self) -> &[String] {
        &self.preview_lines
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected_item(&self) -> Option<&PreviewItem> {
        self.items.get(self.selected)
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<PreviewPanelMsg> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                self.keep_selected_visible(1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                if self.selected + 1 < self.items.len() {
                    self.selected += 1;
                }
                self.keep_selected_visible(1);
                None
            }
            KeyCode::PageUp => {
                let step = self.max_items.unwrap_or(10);
                self.selected = self.selected.saturating_sub(step);
                self.keep_selected_visible(step);
                None
            }
            KeyCode::PageDown => {
                let step = self.max_items.unwrap_or(10);
                self.selected = (self.selected + step).min(self.items.len().saturating_sub(1));
                self.keep_selected_visible(step);
                None
            }
            KeyCode::Home => {
                self.selected = 0;
                self.scroll = 0;
                None
            }
            KeyCode::End => {
                self.selected = self.items.len().saturating_sub(1);
                self.keep_selected_visible(self.max_items.unwrap_or(10));
                None
            }
            KeyCode::Enter => {
                if self.items.is_empty() || self.items[self.selected].disabled {
                    None
                } else {
                    Some(PreviewPanelMsg::Selected(self.selected))
                }
            }
            KeyCode::Esc => Some(PreviewPanelMsg::Cancelled),
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<PreviewPanelMsg> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let local_row = mouse.row.saturating_sub(self.y_offset) as usize;
                let item_row = local_row.checked_sub(self.item_start_row())?;
                let item_count = self.visible_item_count_for_height(usize::MAX);
                if item_row >= item_count {
                    return None;
                }
                let index = self.window_start(item_count).saturating_add(item_row);
                if index < self.items.len() {
                    self.selected = index;
                    if self.items[index].disabled {
                        None
                    } else {
                        Some(PreviewPanelMsg::Selected(index))
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
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

        for (index, item) in self.items.iter().enumerate() {
            let mut text = TextElement::new(self.plain_item_line(index, None));
            if index == self.selected {
                text = text.fg(self.selected_fg).bg(self.selected_bg).bold();
            } else if item.disabled {
                text = text.fg(self.disabled_color);
            } else {
                text = text.fg(item.color.unwrap_or(self.text_color));
            }
            children.push(Element::Text(text));
        }

        if let Some(title) = self
            .preview_title
            .as_deref()
            .filter(|title| !title.is_empty())
        {
            children.push(Element::Text(
                TextElement::new(format!("── {title} ──")).fg(self.divider_color),
            ));
        }
        for line in &self.preview_lines {
            children.push(Element::Text(
                TextElement::new(strip_ansi(line)).fg(self.preview_color),
            ));
        }

        if let Some(footer) = self.footer.as_deref().filter(|footer| !footer.is_empty()) {
            children.push(Element::Text(TextElement::new(footer).fg(self.muted_color)));
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
                    .render(&fit_visible(
                        &format!("{}{}", " ".repeat(self.indent), title),
                        width,
                    )),
            );
        }
        if let Some(subtitle) = self
            .subtitle
            .as_deref()
            .filter(|subtitle| !subtitle.is_empty())
        {
            lines.push(Style::new().fg(self.subtitle_color).render(&fit_visible(
                &format!("{}{}", " ".repeat(self.indent), subtitle),
                width,
            )));
        }

        let visible_items = self.visible_item_count_for_height(height);
        let start = self.window_start(visible_items);
        let end = (start + visible_items).min(self.items.len());
        for index in start..end {
            lines.push(self.render_item(index, width));
        }

        if let Some(title) = self
            .preview_title
            .as_deref()
            .filter(|title| !title.is_empty())
            .filter(|_| lines.len() < height)
        {
            lines.push(self.render_preview_divider(title, width));
        }

        let preview_indent = " ".repeat(self.indent + 2);
        let preview_width = width.saturating_sub(visible_len(&preview_indent));
        let preview_slots = height.saturating_sub(lines.len() + self.footer_rows());
        for line in self.preview_lines.iter().take(preview_slots) {
            lines.push(format!(
                "{preview_indent}{}",
                fit_visible(line, preview_width)
            ));
        }

        if let Some(footer) = self.footer.as_deref().filter(|footer| !footer.is_empty()) {
            lines.push(Style::new().fg(self.muted_color).render(&fit_visible(
                &format!("{}{}", " ".repeat(self.indent), footer),
                width,
            )));
        }

        lines
    }

    fn render_item(&self, index: usize, width: usize) -> String {
        let raw = fit_visible(&self.plain_item_line(index, Some(width)), width);
        let item = &self.items[index];
        if index == self.selected {
            Style::new()
                .fg(self.selected_fg)
                .bg(self.selected_bg)
                .render(&raw)
        } else if item.disabled {
            Style::new().fg(self.disabled_color).render(&raw)
        } else {
            Style::new()
                .fg(item.color.unwrap_or(self.text_color))
                .render(&raw)
        }
    }

    fn plain_item_line(&self, index: usize, width: Option<usize>) -> String {
        let Some(item) = self.items.get(index) else {
            return String::new();
        };
        let marker = if index == self.selected {
            self.marker.as_str()
        } else {
            " "
        };
        let prefix = format!("{}{} ", " ".repeat(self.indent), marker);
        let mut label = item.label.clone();
        if let Some(description) = item
            .description
            .as_deref()
            .filter(|description| !description.is_empty())
        {
            label.push_str("  ");
            label.push_str(description);
        }
        let available = width
            .map(|width| width.saturating_sub(visible_len(&prefix)))
            .unwrap_or(usize::MAX);
        format!("{prefix}{}", truncate_visible(&label, available))
    }

    fn render_preview_divider(&self, title: &str, width: usize) -> String {
        let indent = " ".repeat(self.indent);
        let label = format!("{indent}── {title} ");
        let fill = "─".repeat(width.saturating_sub(visible_len(&label)));
        Style::new()
            .fg(self.divider_color)
            .render(&fit_visible(&format!("{label}{fill}"), width))
    }

    fn visible_item_count_for_height(&self, height: usize) -> usize {
        if self.items.is_empty() {
            return 0;
        }
        let reserved = self.item_start_row()
            + self.preview_title_rows()
            + self.preview_lines.len()
            + self.footer_rows();
        let available = height.saturating_sub(reserved).max(1);
        self.max_items.unwrap_or(available).min(available)
    }

    fn window_start(&self, visible_items: usize) -> usize {
        if visible_items == 0 || self.items.len() <= visible_items {
            return 0;
        }
        let max_start = self.items.len().saturating_sub(visible_items);
        let mut start = self.scroll.min(max_start);
        if self.selected < start {
            start = self.selected;
        } else if self.selected >= start + visible_items {
            start = self.selected + 1 - visible_items;
        }
        start.min(max_start)
    }

    fn keep_selected_visible(&mut self, window_hint: usize) {
        let visible_items = self.max_items.unwrap_or(window_hint.max(1));
        self.scroll = self.window_start(visible_items);
    }

    fn item_start_row(&self) -> usize {
        usize::from(self.title.as_ref().is_some_and(|title| !title.is_empty()))
            + usize::from(
                self.subtitle
                    .as_ref()
                    .is_some_and(|subtitle| !subtitle.is_empty()),
            )
    }

    fn preview_title_rows(&self) -> usize {
        usize::from(
            self.preview_title
                .as_ref()
                .is_some_and(|title| !title.is_empty()),
        )
    }

    fn footer_rows(&self) -> usize {
        usize::from(
            self.footer
                .as_ref()
                .is_some_and(|footer| !footer.is_empty()),
        )
    }

    fn clamp_selection(&mut self) {
        self.selected = self.selected.min(self.items.len().saturating_sub(1));
    }
}

impl Default for PreviewPanel {
    fn default() -> Self {
        Self::without_title()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    fn sample() -> PreviewPanel {
        PreviewPanel::new("Theme")
            .subtitle("Enter apply · Esc cancel")
            .items(vec![
                PreviewItem::new("Atom One Dark").description("default"),
                PreviewItem::new("Ayu Mirage").color(Color::Yellow),
                PreviewItem::new("Quiet Light").disabled(true),
            ])
            .selected(1)
            .preview_title("syntax preview")
            .preview_lines(vec![
                "// syntax preview",
                "fn compute(n: usize) -> String {",
                "    format!(\"sum: {}\", n)",
                "}",
            ])
            .footer("↑/↓ preview")
    }

    #[test]
    fn renders_items_and_preview_lines() {
        let rendered = sample().view(48, 10);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("Theme"));
        assert!(plain.contains("Ayu Mirage"));
        assert!(plain.contains("syntax preview"));
        assert!(plain.contains("fn compute"));
        assert!(plain.contains("↑/↓ preview"));
        assert_eq!(rendered.lines().count(), 10);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 48, "{line:?}");
        }
    }

    #[test]
    fn keeps_selected_item_visible_when_scrolled() {
        let items = (0..12)
            .map(|index| PreviewItem::new(format!("Theme {index}")))
            .collect::<Vec<_>>();
        let rendered = PreviewPanel::new("Theme")
            .items(items)
            .selected(10)
            .max_items(4)
            .preview_line("sample")
            .view(32, 7);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("Theme 10"), "{plain:?}");
        assert!(!plain.contains("Theme 0"), "{plain:?}");
    }

    #[test]
    fn preview_accepts_ansi_and_cjk_without_overflow() {
        let rendered = PreviewPanel::new("Theme")
            .item(PreviewItem::new("Atom"))
            .preview_line(
                Style::new()
                    .fg(Color::Cyan)
                    .render("中文预览 content with a long tail"),
            )
            .view(24, 4);

        assert!(rendered.contains("\x1b["));
        assert!(strip_ansi(&rendered).contains("中文"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 24, "{line:?}");
        }
    }

    #[test]
    fn handle_key_selects_and_cancels() {
        let mut panel = sample();
        assert_eq!(panel.selected_index(), 1);

        let down = KeyEvent {
            code: KeyCode::Down,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        assert_eq!(panel.handle_key(&down), None);
        assert_eq!(panel.selected_index(), 2);

        let enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        assert_eq!(panel.handle_key(&enter), None);

        let esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        assert_eq!(panel.handle_key(&esc), Some(PreviewPanelMsg::Cancelled));
    }

    #[test]
    fn zero_size_renders_empty_string() {
        assert_eq!(sample().view(0, 8), "");
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
