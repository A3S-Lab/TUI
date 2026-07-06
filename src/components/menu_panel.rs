use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::interaction::{Activatable, Scrollable, Selectable};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};
use crossterm::event::KeyCode;

const MAX_MENU_ITEM_DEPTH: usize = u16::MAX as usize / 2;
const MAX_MENU_PANEL_INDENT: usize = u16::MAX as usize;
const MAX_MENU_PANEL_LABEL_WIDTH: usize = u16::MAX as usize;
const MAX_MENU_PANEL_ITEMS: usize = u16::MAX as usize;

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
        self.depth = depth.min(MAX_MENU_ITEM_DEPTH);
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
        self.max_items = Some(max_items.clamp(1, MAX_MENU_PANEL_ITEMS));
        self
    }

    pub fn label_width(mut self, width: usize) -> Self {
        self.label_width = Some(width.min(MAX_MENU_PANEL_LABEL_WIDTH));
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
        self.indent = indent.min(MAX_MENU_PANEL_INDENT);
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

    /// Apply semantic colors from a theme while preserving content and layout.
    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.title_color = theme.color(ThemeRole::Primary);
        self.subtitle_color = theme.color(ThemeRole::Muted);
        self.text_color = theme.color(ThemeRole::Foreground);
        self.muted_color = theme.color(ThemeRole::Muted);
        self.selected_fg = theme.color(ThemeRole::Foreground);
        self.selected_bg = theme.color(ThemeRole::Highlight);
        self.checked_color = theme.color(ThemeRole::Success);
        self.disabled_color = theme.color(ThemeRole::Muted);
        self
    }

    pub fn set_y_offset(&mut self, y: u16) {
        self.y_offset = y;
    }

    pub fn items_value(&self) -> &[MenuItem] {
        &self.items
    }

    pub fn selected_index(&self) -> usize {
        self.normalized_selected()
    }

    pub fn selected_item(&self) -> Option<&MenuItem> {
        self.items.get(self.normalized_selected())
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<MenuPanelMsg> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.normalized_selected().saturating_sub(1);
                self.keep_selected_visible(1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                let selected = self.normalized_selected();
                if selected.saturating_add(1) < self.items.len() {
                    self.selected = selected + 1;
                } else {
                    self.selected = selected;
                }
                self.keep_selected_visible(1);
                None
            }
            KeyCode::PageUp => {
                let step = self.max_items.unwrap_or(10);
                self.selected = self.normalized_selected().saturating_sub(step);
                self.keep_selected_visible(step);
                None
            }
            KeyCode::PageDown => {
                let step = self.max_items.unwrap_or(10);
                self.selected = self
                    .normalized_selected()
                    .saturating_add(step)
                    .min(self.items.len().saturating_sub(1));
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
            MouseEventKind::ScrollUp => {
                super::relative_mouse_row(mouse.row, self.y_offset)?;
                self.selected = self.normalized_selected().saturating_sub(1);
                self.keep_selected_visible(1);
                None
            }
            MouseEventKind::ScrollDown => {
                super::relative_mouse_row(mouse.row, self.y_offset)?;
                let selected = self.normalized_selected();
                self.selected = selected
                    .saturating_add(1)
                    .min(self.items.len().saturating_sub(1));
                self.keep_selected_visible(1);
                None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let local_row = super::relative_mouse_row(mouse.row, self.y_offset)?;
                let item_row = local_row.checked_sub(self.item_start_row())?;
                let item_count = self.visible_item_count_for_height(usize::MAX);
                if item_row >= item_count {
                    return None;
                }
                let index = self.window_start(item_count).saturating_add(item_row);
                if index < self.items.len() {
                    self.selected = index;
                    self.clicked_msg()
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

        let selected = self.normalized_selected();
        for index in self.element_item_range() {
            children.push(Element::Text(self.item_text_element(index, selected)));
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

    pub fn element_with_height<Msg>(&self, height: usize) -> Element<Msg> {
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

        let visible_items = self.visible_item_count_for_height(height);
        let start = self.window_start(visible_items);
        let end = start.saturating_add(visible_items).min(self.items.len());
        let selected = self.normalized_selected();
        for index in start..end {
            children.push(Element::Text(self.item_text_element(index, selected)));
        }

        if self.show_scroll && self.items.len() > visible_items && visible_items > 0 {
            children.push(Element::Text(self.scroll_footer_element(start, end)));
        }

        if let Some(footer) = self.footer.as_deref().filter(|footer| !footer.is_empty()) {
            children.push(Element::Text(TextElement::new(footer).fg(self.muted_color)));
        }

        children.truncate(height);
        if self.fill_height {
            while children.len() < height {
                children.push(Element::Text(TextElement::new("")));
            }
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
                        &format!("{}{}", " ".repeat(self.indent_for_width(width)), title),
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
                &format!("{}{}", " ".repeat(self.indent_for_width(width)), subtitle),
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
                &format!("{}{}", " ".repeat(self.indent_for_width(width)), footer),
                width,
            )));
        }

        lines
    }

    fn item_text_element(&self, index: usize, selected: usize) -> TextElement {
        let item = &self.items[index];
        let mut text = TextElement::new(self.plain_item_line(index, None));
        if index == selected {
            text = text.fg(self.selected_fg).bg(self.selected_bg).bold();
        } else if item.disabled {
            text = text.fg(self.disabled_color);
        } else {
            text = text.fg(self.item_color(item));
        }
        text
    }

    fn scroll_footer_element(&self, start: usize, end: usize) -> TextElement {
        let up = if start > 0 { "↑" } else { " " };
        let down = if end < self.items.len() { "↓" } else { " " };
        TextElement::new(format!(
            "{}{up}{down} {}/{}",
            " ".repeat(self.indent_for_element()),
            self.normalized_selected()
                .saturating_add(1)
                .min(self.items.len()),
            self.items.len()
        ))
        .fg(self.muted_color)
    }

    fn render_item(&self, index: usize, width: usize) -> String {
        let raw = fit_visible(&self.plain_item_line(index, Some(width)), width);
        let item = &self.items[index];
        if index == self.normalized_selected() {
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
        let marker = if index == self.normalized_selected() {
            self.marker.as_str()
        } else {
            " "
        };
        let prefix = match width {
            Some(width) => self.item_prefix_for_width(index, item, marker, width),
            None => self.item_prefix_for_element(index, item, marker),
        };

        let mut label = item.label.clone();
        if let Some(label_width) = self.label_width {
            let label_width = label_width
                .min(MAX_MENU_PANEL_LABEL_WIDTH)
                .min(width.unwrap_or(label_width));
            label = fit_visible(&label, label_width);
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
                " ".repeat(self.indent_for_width(width)),
                self.normalized_selected()
                    .saturating_add(1)
                    .min(self.items.len()),
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
        let selected = self.normalized_selected();
        if selected < start {
            start = selected;
        } else if selected >= start + visible_items {
            start = selected + 1 - visible_items;
        }
        start.min(max_start)
    }

    fn element_item_range(&self) -> std::ops::Range<usize> {
        let visible_items = self
            .max_items
            .unwrap_or(self.items.len())
            .min(self.items.len());
        let start = self.window_start(visible_items);
        let end = start.saturating_add(visible_items).min(self.items.len());
        start..end
    }

    fn keep_selected_visible(&mut self, window_hint: usize) {
        let visible_items = self.max_items.unwrap_or(window_hint.max(1));
        self.scroll = self.window_start(visible_items);
    }

    fn selected_msg(&self) -> Option<MenuPanelMsg> {
        let selected = self.normalized_selected();
        let item = self.items.get(selected)?;
        if item.disabled {
            None
        } else {
            Some(MenuPanelMsg::Selected(selected))
        }
    }

    fn selected_toggle_msg(&self) -> Option<MenuPanelMsg> {
        let selected = self.normalized_selected();
        let item = self.items.get(selected)?;
        if item.disabled {
            None
        } else {
            Some(MenuPanelMsg::Toggled(selected))
        }
    }

    fn clicked_msg(&self) -> Option<MenuPanelMsg> {
        let selected = self.normalized_selected();
        let item = self.items.get(selected)?;
        if item.checked.is_some() {
            self.selected_toggle_msg()
        } else {
            self.selected_msg()
        }
    }

    fn number_shortcut_msg(&mut self, c: char) -> Option<MenuPanelMsg> {
        let index = number_shortcut_index(c)?;
        if index < self.items.len() {
            self.selected = index;
            self.keep_selected_visible(self.max_items.unwrap_or(10));
            if self.items[index].disabled {
                None
            } else {
                Some(MenuPanelMsg::Selected(index))
            }
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
        self.selected = self.normalized_selected();
    }

    fn normalized_selected(&self) -> usize {
        self.selected.min(self.items.len().saturating_sub(1))
    }

    fn item_color(&self, item: &MenuItem) -> Color {
        item.color.unwrap_or(if item.checked == Some(true) {
            self.checked_color
        } else {
            self.text_color
        })
    }

    fn indent_for_width(&self, width: usize) -> usize {
        self.indent.min(width).min(MAX_MENU_PANEL_INDENT)
    }

    fn item_prefix_for_width(
        &self,
        index: usize,
        item: &MenuItem,
        marker: &str,
        width: usize,
    ) -> String {
        let tail = truncate_visible(&self.item_prefix_tail(index, item, marker), width);
        let tail_width = visible_len(&tail);
        let indent = self.indent.min(width.saturating_sub(tail_width));
        let depth_width = item
            .depth
            .saturating_mul(2)
            .min(width.saturating_sub(indent).saturating_sub(tail_width));
        format!("{}{}{}", " ".repeat(indent), " ".repeat(depth_width), tail)
    }

    fn item_prefix_for_element(&self, index: usize, item: &MenuItem, marker: &str) -> String {
        format!(
            "{}{}{}",
            " ".repeat(self.indent_for_element()),
            "  ".repeat(item.depth.min(MAX_MENU_ITEM_DEPTH)),
            self.item_prefix_tail(index, item, marker)
        )
    }

    fn item_prefix_tail(&self, index: usize, item: &MenuItem, marker: &str) -> String {
        let mut prefix = format!("{marker} ");
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
        prefix
    }

    fn indent_for_element(&self) -> usize {
        self.indent.min(MAX_MENU_PANEL_INDENT)
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

impl Selectable for MenuPanel {
    fn item_count(&self) -> usize {
        self.items.len()
    }

    fn selected_index(&self) -> Option<usize> {
        (!self.items.is_empty()).then(|| self.normalized_selected())
    }

    fn select_index(&mut self, index: usize) {
        self.selected = index;
        self.clamp_selection();
    }
}

impl Scrollable for MenuPanel {
    fn scroll_offset(&self) -> usize {
        self.scroll
    }

    fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll = offset.min(self.items.len().saturating_sub(1));
    }
}

impl Activatable for MenuPanel {
    fn is_item_disabled(&self, index: usize) -> bool {
        self.items.get(index).is_some_and(MenuItem::is_disabled)
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
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let panel = MenuPanel::without_title().with_theme(&theme);

        assert_eq!(panel.title_color, theme.color(ThemeRole::Primary));
        assert_eq!(panel.text_color, theme.color(ThemeRole::Foreground));
        assert_eq!(panel.muted_color, theme.color(ThemeRole::Muted));
        assert_eq!(panel.selected_bg, theme.color(ThemeRole::Highlight));
        assert_eq!(panel.checked_color, theme.color(ThemeRole::Success));
        assert_eq!(panel.disabled_color, theme.color(ThemeRole::Muted));
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
    fn oversized_spacing_is_clamped_to_render_width() {
        let panel = MenuPanel::new("Commands")
            .subtitle("hint")
            .indent(usize::MAX)
            .label_width(usize::MAX)
            .item(MenuItem::new("run").depth(usize::MAX).description("desc"))
            .footer("footer")
            .fill_height(true);
        let rendered = panel.view(8, 4);
        let item = panel.items.first().unwrap();
        let line = panel.plain_item_line(0, Some(8));
        let prefix = panel.item_prefix_for_width(0, item, "▸", 8);

        assert_eq!(panel.indent, MAX_MENU_PANEL_INDENT);
        assert_eq!(panel.label_width, Some(MAX_MENU_PANEL_LABEL_WIDTH));
        assert_eq!(item.depth, MAX_MENU_ITEM_DEPTH);
        assert_eq!(panel.indent_for_width(8), 8);
        assert_eq!(visible_len(&prefix), 8);
        assert_eq!(visible_len(&line), 8);
        assert!(rendered.lines().all(|line| visible_len(line) == 8));
    }

    #[test]
    fn oversized_item_limit_is_clamped() {
        let panel = MenuPanel::new("Commands")
            .max_items(usize::MAX)
            .item(MenuItem::new("one"))
            .item(MenuItem::new("two"));
        let rendered = panel.view(24, 4);

        assert_eq!(panel.max_items, Some(MAX_MENU_PANEL_ITEMS));
        assert!(rendered.lines().all(|line| visible_len(line) == 24));
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
    fn huge_page_down_saturates_selection() {
        let mut panel = sample_panel().selected(1).max_items(usize::MAX);

        assert_eq!(panel.handle_key(&key(KeyCode::PageDown)), None);

        assert_eq!(panel.selected_index(), panel.items_value().len() - 1);
    }

    #[test]
    fn stale_selection_is_normalized_for_rendering_and_input() {
        let mut panel = MenuPanel::new("Menu")
            .max_items(1)
            .item(MenuItem::new("one"))
            .item(MenuItem::new("two"));
        panel.selected = usize::MAX;

        assert_eq!(panel.selected_index(), 1);
        assert_eq!(panel.selected_item().map(MenuItem::label), Some("two"));
        assert_eq!(
            panel.handle_key(&key(KeyCode::Enter)),
            Some(MenuPanelMsg::Selected(1))
        );
        assert_eq!(
            panel.handle_key(&key(KeyCode::Char(' '))),
            Some(MenuPanelMsg::Toggled(1))
        );

        let plain = strip_ansi(&panel.view(20, 4));
        assert!(plain.contains("▸ two"));
        assert!(!plain.contains("▸ one"));

        let Element::Box(box_el) = panel.element::<()>() else {
            panic!("expected box element");
        };
        assert_eq!(box_el.children.len(), 2);
        let Element::Text(last_item) = box_el.children.last().expect("expected last item") else {
            panic!("expected menu item");
        };
        assert_eq!(last_item.content, "  ▸ two");

        assert_eq!(panel.handle_key(&key(KeyCode::Down)), None);
        assert_eq!(panel.selected_index(), 1);
        assert_eq!(panel.handle_key(&key(KeyCode::Up)), None);
        assert_eq!(panel.selected_index(), 0);
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
    fn disabled_items_do_not_emit_actions() {
        let mut panel = sample_panel().selected(4).number_shortcuts(true);

        assert_eq!(panel.handle_key(&key(KeyCode::Enter)), None);
        assert_eq!(panel.handle_key(&key(KeyCode::Char(' '))), None);
        assert_eq!(panel.handle_key(&key(KeyCode::Char('5'))), None);
        assert_eq!(panel.selected_index(), 4);
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

        assert_eq!(msg, Some(MenuPanelMsg::Toggled(2)));
        assert_eq!(panel.selected_index(), 2);
    }

    #[test]
    fn mouse_click_on_plain_row_selects_it() {
        let mut panel = sample_panel();
        let msg = panel.handle_mouse(&MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 2,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(msg, Some(MenuPanelMsg::Selected(0)));
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn mouse_wheel_updates_selected_menu_item() {
        let mut panel = sample_panel();

        assert_eq!(
            panel.handle_mouse(&MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 0,
                row: 2,
                modifiers: KeyModifiers::NONE,
            }),
            None
        );
        assert_eq!(panel.selected_index(), 1);

        assert_eq!(
            panel.handle_mouse(&MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 0,
                row: 2,
                modifiers: KeyModifiers::NONE,
            }),
            None
        );
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn mouse_click_above_offset_is_ignored() {
        let mut panel = sample_panel().selected(2).scroll(1);
        panel.set_y_offset(4);

        let msg = panel.handle_mouse(&MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 3,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(msg, None);
        assert_eq!(panel.selected_index(), 2);
    }

    #[test]
    fn mouse_click_on_disabled_item_only_moves_selection() {
        let mut panel = sample_panel().selected(4);
        let msg = panel.handle_mouse(&MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 4,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(msg, None);
        assert_eq!(panel.selected_index(), 4);
    }

    #[test]
    fn element_produces_column() {
        let el: Element<()> = sample_panel().element();

        match el {
            Element::Box(column) => {
                assert_eq!(column.style.flex_direction, FlexDirection::Column);
                assert_eq!(column.children.len(), 6);
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn element_respects_max_items_window() {
        let el: Element<()> = sample_panel().selected(3).scroll(1).element();

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        let text = column
            .children
            .iter()
            .filter_map(Element::text_content)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(text.contains("/theme"));
        assert!(text.contains("/plugins"));
        assert!(text.contains("/top"));
        assert!(!text.contains("/model"));
        assert!(!text.contains("/quit"));
    }

    #[test]
    fn element_with_height_zero_returns_empty_column() {
        let el: Element<()> = sample_panel().element_with_height(0);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        assert_eq!(column.style.flex_direction, FlexDirection::Column);
        assert!(column.children.is_empty());
    }

    #[test]
    fn element_with_height_limits_items_and_keeps_scroll_footer() {
        let el: Element<()> = sample_panel().selected(3).scroll(1).element_with_height(5);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        assert_eq!(column.children.len(), 5);
        let text = column
            .children
            .iter()
            .filter_map(Element::text_content)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(text.contains("Commands"));
        assert!(text.contains("Enter run"));
        assert!(text.contains("/top"));
        assert!(text.contains("↑↓ 4/5"));
        assert!(text.contains("type to filter"));
        assert!(!text.contains("/model"));
        assert!(!text.contains("/theme"));
    }

    #[test]
    fn element_with_height_fill_height_pads_empty_rows() {
        let el: Element<()> = MenuPanel::without_title()
            .item(MenuItem::new("one"))
            .fill_height(true)
            .element_with_height(3);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        assert_eq!(column.children.len(), 3);
        assert!(column.children[0]
            .text_content()
            .is_some_and(|text| text.contains("one")));
        assert_eq!(column.children[1].text_content(), Some(""));
        assert_eq!(column.children[2].text_content(), Some(""));
    }
}
