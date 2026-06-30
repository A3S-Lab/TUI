use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};
use crossterm::event::KeyCode;

/// One row in a [`MenuPanel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    label: String,
    description: Option<String>,
    prefix: Option<String>,
    suffix: Option<String>,
    checked: Option<bool>,
    depth: usize,
    color: Option<Color>,
    disabled: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            prefix: None,
            suffix: None,
            checked: None,
            depth: 0,
            color: None,
            disabled: false,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
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

    pub fn is_checked(&self) -> Option<bool> {
        self.checked
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// Message returned by [`MenuPanel`] input handlers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuPanelMsg {
    Selected(usize),
    Toggled(usize),
    Cancelled,
}

/// A titled, scroll-aware selectable menu for command palettes and overlays.
///
/// This component covers the line-rendered menus commonly used in terminal
/// applications: a title, optional subtitle, a highlighted selected row,
/// descriptions, checkboxes, indentation for tree-like items, and scroll
/// position hints.
#[derive(Debug, Clone)]
pub struct MenuPanel {
    title: Option<String>,
    subtitle: Option<String>,
    items: Vec<MenuItem>,
    selected: usize,
    scroll: usize,
    max_items: Option<usize>,
    label_width: Option<usize>,
    show_scroll: bool,
    number_shortcuts: bool,
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
    checked_color: Color,
    disabled_color: Color,
    footer: Option<String>,
}

impl MenuPanel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            subtitle: None,
            items: Vec::new(),
            selected: 0,
            scroll: 0,
            max_items: None,
            label_width: None,
            show_scroll: true,
            number_shortcuts: false,
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
            checked_color: Color::Green,
            disabled_color: Color::BrightBlack,
            footer: None,
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

    pub fn item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self.clamp_selection();
        self
    }

    pub fn items(mut self, items: Vec<MenuItem>) -> Self {
        self.items = items;
        self.clamp_selection();
        self
    }

    pub fn add_item(&mut self, item: MenuItem) {
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

    pub fn label_width(mut self, width: usize) -> Self {
        self.label_width = Some(width);
        self
    }

    pub fn show_scroll(mut self, enabled: bool) -> Self {
        self.show_scroll = enabled;
        self
    }

    pub fn number_shortcuts(mut self, enabled: bool) -> Self {
        self.number_shortcuts = enabled;
        self
    }

    pub fn fill_height(mut self, enabled: bool) -> Self {
        self.fill_height = enabled;
        self
    }

    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn marker(mut self, marker: impl Into<String>) -> Self {
        self.marker = marker.into();
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

    pub fn checked_color(mut self, color: Color) -> Self {
        self.checked_color = color;
        self
    }

    pub fn disabled_color(mut self, color: Color) -> Self {
        self.disabled_color = color;
        self
    }

    pub fn set_y_offset(&mut self, y: u16) {
        self.y_offset = y;
    }

    pub fn items_value(&self) -> &[MenuItem] {
        &self.items
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected_item(&self) -> Option<&MenuItem> {
        self.items.get(self.selected)
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<MenuPanelMsg> {
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
            KeyCode::Enter => self.selected_msg(),
            KeyCode::Char(' ') => self.selected_toggle_msg(),
            KeyCode::Esc => Some(MenuPanelMsg::Cancelled),
            KeyCode::Char(c) if self.number_shortcuts => self.number_shortcut_msg(c),
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<MenuPanelMsg> {
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
                    Some(MenuPanelMsg::Selected(index))
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
                text = text.fg(self.item_color(item));
            }
            children.push(Element::Text(text));
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

        if self.show_scroll && self.items.len() > visible_items && visible_items > 0 {
            lines.push(self.render_scroll_footer(start, end, width));
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
            Style::new().fg(self.item_color(item)).render(&raw)
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
        let mut prefix = format!(
            "{}{}{} ",
            " ".repeat(self.indent),
            "  ".repeat(item.depth),
            marker
        );
        if self.number_shortcuts {
            if let Some(shortcut) = number_shortcut_label(index) {
                prefix.push_str(&format!("{shortcut}. "));
            } else {
                prefix.push_str("   ");
            }
        }
        if let Some(checked) = item.checked {
            prefix.push_str(if checked { "[✓] " } else { "[ ] " });
        }
        if let Some(extra) = item.prefix.as_deref().filter(|prefix| !prefix.is_empty()) {
            prefix.push_str(extra);
            prefix.push(' ');
        }

        let mut label = item.label.clone();
        if let Some(width) = self.label_width {
            label = fit_visible(&label, width);
        }
        if let Some(suffix) = item.suffix.as_deref().filter(|suffix| !suffix.is_empty()) {
            label.push(' ');
            label.push_str(suffix);
        }
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

    fn render_scroll_footer(&self, start: usize, end: usize, width: usize) -> String {
        let up = if start > 0 { "↑" } else { " " };
        let down = if end < self.items.len() { "↓" } else { " " };
        Style::new().fg(self.muted_color).render(&fit_visible(
            &format!(
                "{}{up}{down} {}/{}",
                " ".repeat(self.indent),
                self.selected.saturating_add(1).min(self.items.len()),
                self.items.len()
            ),
            width,
        ))
    }

    fn visible_item_count_for_height(&self, height: usize) -> usize {
        if self.items.is_empty() {
            return 0;
        }
        let reserved = self.item_start_row()
            + usize::from(
                self.footer
                    .as_ref()
                    .is_some_and(|footer| !footer.is_empty()),
            )
            + usize::from(self.show_scroll && self.items.len() > 1);
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

    fn selected_msg(&self) -> Option<MenuPanelMsg> {
        if self.items.is_empty() {
            None
        } else {
            Some(MenuPanelMsg::Selected(self.selected))
        }
    }

    fn selected_toggle_msg(&self) -> Option<MenuPanelMsg> {
        if self.items.is_empty() {
            None
        } else {
            Some(MenuPanelMsg::Toggled(self.selected))
        }
    }

    fn number_shortcut_msg(&mut self, c: char) -> Option<MenuPanelMsg> {
        let index = number_shortcut_index(c)?;
        if index < self.items.len() {
            self.selected = index;
            self.keep_selected_visible(self.max_items.unwrap_or(10));
            Some(MenuPanelMsg::Selected(index))
        } else {
            None
        }
    }

    fn item_start_row(&self) -> usize {
        usize::from(self.title.as_ref().is_some_and(|title| !title.is_empty()))
            + usize::from(
                self.subtitle
                    .as_ref()
                    .is_some_and(|subtitle| !subtitle.is_empty()),
            )
    }

    fn clamp_selection(&mut self) {
        self.selected = self.selected.min(self.items.len().saturating_sub(1));
    }

    fn item_color(&self, item: &MenuItem) -> Color {
        item.color.unwrap_or(if item.checked == Some(true) {
            self.checked_color
        } else {
            self.text_color
        })
    }
}

impl Default for MenuPanel {
    fn default() -> Self {
        Self::without_title()
    }
}

fn number_shortcut_index(c: char) -> Option<usize> {
    match c {
        '1'..='9' => Some((c as u8 - b'1') as usize),
        '0' => Some(9),
        _ => None,
    }
}

fn number_shortcut_label(index: usize) -> Option<char> {
    match index {
        0..=8 => Some((b'1' + index as u8) as char),
        9 => Some('0'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::strip_ansi;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn sample_panel() -> MenuPanel {
        MenuPanel::new("Commands")
            .subtitle("Enter run · Esc close")
            .label_width(10)
            .max_items(3)
            .item(MenuItem::new("/model").description("pick the model"))
            .item(MenuItem::new("/theme").description("preview colors"))
            .item(
                MenuItem::new("/plugins")
                    .checked(true)
                    .description("toggle skills"),
            )
            .item(MenuItem::new("/top").description("process monitor"))
            .item(MenuItem::new("/quit").disabled(true).description("leave"))
            .footer("type to filter")
    }

    #[test]
    fn renders_title_subtitle_items_and_scroll_footer() {
        let rendered = sample_panel().selected(3).view(44, 7);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("Commands"));
        assert!(plain.contains("Enter run"));
        assert!(plain.contains("/theme"));
        assert!(plain.contains("/top"));
        assert!(plain.contains("↑↓ 4/5"));
        assert_eq!(rendered.lines().count(), 7);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 44, "{line:?}");
        }
    }

    #[test]
    fn truncates_cjk_descriptions_to_width() {
        let panel = MenuPanel::new("Files")
            .item(MenuItem::new("src/main.rs").description("中文测试内容 with a long suffix"));
        let rendered = panel.view(24, 4);

        for line in rendered.lines() {
            assert_eq!(visible_len(line), 24, "{line:?}");
        }
        assert!(strip_ansi(&rendered).contains("中文"));
    }

    #[test]
    fn key_handling_moves_selects_toggles_and_cancels() {
        let mut panel = sample_panel();

        panel.handle_key(&key(KeyCode::Down));
        assert_eq!(panel.selected_index(), 1);
        assert_eq!(
            panel.handle_key(&key(KeyCode::Enter)),
            Some(MenuPanelMsg::Selected(1))
        );
        assert_eq!(
            panel.handle_key(&key(KeyCode::Char(' '))),
            Some(MenuPanelMsg::Toggled(1))
        );
        assert_eq!(
            panel.handle_key(&key(KeyCode::Esc)),
            Some(MenuPanelMsg::Cancelled)
        );
    }

    #[test]
    fn number_shortcuts_are_opt_in() {
        let mut panel = sample_panel().number_shortcuts(true);

        assert_eq!(
            panel.handle_key(&key(KeyCode::Char('3'))),
            Some(MenuPanelMsg::Selected(2))
        );
        assert_eq!(panel.selected_index(), 2);
    }

    #[test]
    fn mouse_click_selects_visible_row() {
        let mut panel = sample_panel().selected(2).scroll(1);
        panel.set_y_offset(4);
        let msg = panel.handle_mouse(&MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 7,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(msg, Some(MenuPanelMsg::Selected(2)));
        assert_eq!(panel.selected_index(), 2);
    }

    #[test]
    fn element_produces_column() {
        let el: Element<()> = sample_panel().element();

        match el {
            Element::Box(column) => {
                assert_eq!(column.style.flex_direction, FlexDirection::Column);
                assert_eq!(column.children.len(), 8);
            }
            _ => panic!("expected Box"),
        }
    }
}
