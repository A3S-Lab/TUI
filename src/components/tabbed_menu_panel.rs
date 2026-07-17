use super::chip_strip::{Chip, ChipStrip};
use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::interaction::{Activatable, Scrollable, Selectable, Tabbed};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};
use crossterm::event::KeyCode;

const MAX_TABBED_MENU_PANEL_INDENT: usize = u16::MAX as usize;
const MAX_TABBED_MENU_PANEL_ITEMS: usize = u16::MAX as usize;
const MAX_TABBED_MENU_PANEL_TAB_GAP: usize = u16::MAX as usize;

/// One selectable row in a [`TabbedMenuPanel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabbedMenuItem {
    label: String,
    description: Option<String>,
    prefix: Option<String>,
    color: Option<Color>,
    disabled: bool,
}

impl TabbedMenuItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            prefix: None,
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

    pub fn prefix_value(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    pub fn color_value(&self) -> Option<Color> {
        self.color
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// One tab/source in a [`TabbedMenuPanel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabbedMenuTab {
    label: String,
    color: Color,
    items: Vec<TabbedMenuItem>,
    empty_text: Option<String>,
}

impl TabbedMenuTab {
    pub fn new(label: impl Into<String>, color: Color) -> Self {
        Self {
            label: label.into(),
            color,
            items: Vec::new(),
            empty_text: None,
        }
    }

    pub fn item(mut self, item: TabbedMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: Vec<TabbedMenuItem>) -> Self {
        self.items = items;
        self
    }

    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty_text = Some(text.into());
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn color_value(&self) -> Color {
        self.color
    }

    pub fn items_value(&self) -> &[TabbedMenuItem] {
        &self.items
    }

    pub fn empty_text_value(&self) -> Option<&str> {
        self.empty_text.as_deref()
    }
}

/// Message returned by [`TabbedMenuPanel`] input handlers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabbedMenuPanelMsg {
    TabChanged(usize),
    Selected { tab: usize, item: usize },
    Cancelled,
}

/// Colored tab strip plus a scroll-aware selectable list.
///
/// This extracts the tabbed picker pattern from the CLI: colored source
/// chips, optional title and hint rows, active-tab switching, and a selected
/// list whose window follows the cursor.
#[derive(Debug, Clone)]
pub struct TabbedMenuPanel {
    title: Option<String>,
    hint: Option<String>,
    tabs: Vec<TabbedMenuTab>,
    active_tab: usize,
    selected: usize,
    scroll: usize,
    max_items: Option<usize>,
    show_tabs_when_single: bool,
    show_scroll: bool,
    fill_height: bool,
    y_offset: u16,
    indent: usize,
    tab_gap: usize,
    title_color: Option<Color>,
    hint_color: Color,
    text_color: Color,
    muted_color: Color,
    selected_fg: Color,
    selected_bg: Option<Color>,
    disabled_color: Color,
    items_use_tab_color: bool,
    inactive_tabs_use_tab_color: bool,
    active_tab_foreground: Option<Color>,
    footer: Option<String>,
}

impl TabbedMenuPanel {
    pub fn new(tabs: Vec<TabbedMenuTab>) -> Self {
        Self {
            title: None,
            hint: None,
            tabs,
            active_tab: 0,
            selected: 0,
            scroll: 0,
            max_items: None,
            show_tabs_when_single: false,
            show_scroll: true,
            fill_height: false,
            y_offset: 0,
            indent: 2,
            tab_gap: 1,
            title_color: None,
            hint_color: Color::BrightBlack,
            text_color: Color::BrightBlack,
            muted_color: Color::BrightBlack,
            selected_fg: Color::Black,
            selected_bg: None,
            disabled_color: Color::BrightBlack,
            items_use_tab_color: false,
            inactive_tabs_use_tab_color: false,
            active_tab_foreground: None,
            footer: None,
        }
        .clamped()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn tabs(mut self, tabs: Vec<TabbedMenuTab>) -> Self {
        self.tabs = tabs;
        self.clamp_state();
        self
    }

    pub fn active_tab(mut self, active_tab: usize) -> Self {
        self.active_tab = active_tab;
        self.clamp_state();
        self
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self.clamp_state();
        self
    }

    pub fn scroll(mut self, scroll: usize) -> Self {
        self.scroll = scroll;
        self
    }

    pub fn max_items(mut self, max_items: usize) -> Self {
        self.max_items = Some(max_items.clamp(1, MAX_TABBED_MENU_PANEL_ITEMS));
        self
    }

    pub fn show_tabs_when_single(mut self, enabled: bool) -> Self {
        self.show_tabs_when_single = enabled;
        self
    }

    pub fn show_scroll(mut self, enabled: bool) -> Self {
        self.show_scroll = enabled;
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
        self.indent = indent.min(MAX_TABBED_MENU_PANEL_INDENT);
        self
    }

    pub fn tab_gap(mut self, gap: usize) -> Self {
        self.tab_gap = gap.min(MAX_TABBED_MENU_PANEL_TAB_GAP);
        self
    }

    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = Some(color);
        self
    }

    pub fn hint_color(mut self, color: Color) -> Self {
        self.hint_color = color;
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
        self.selected_bg = Some(bg);
        self
    }

    pub fn selected_fg(mut self, color: Color) -> Self {
        self.selected_fg = color;
        self
    }

    pub fn disabled_color(mut self, color: Color) -> Self {
        self.disabled_color = color;
        self
    }

    pub fn items_use_tab_color(mut self, enabled: bool) -> Self {
        self.items_use_tab_color = enabled;
        self
    }

    /// Render inactive tabs with their configured tab colors.
    ///
    /// Inactive tabs use [`Self::muted_color`] by default. Enable this for
    /// source pickers where every tab color communicates provider identity.
    pub fn inactive_tabs_use_tab_color(mut self, enabled: bool) -> Self {
        self.inactive_tabs_use_tab_color = enabled;
        self
    }

    /// Set the active tab's foreground without changing selected item text.
    ///
    /// By default the active tab shares [`Self::selected_fg`]. A separate
    /// foreground keeps bright brand-color tab backgrounds readable while
    /// selected list rows retain their own foreground.
    pub fn active_tab_foreground(mut self, color: Color) -> Self {
        self.active_tab_foreground = Some(color);
        self
    }

    /// Apply semantic colors from a theme while preserving tab brand colors.
    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.title_color = Some(theme.color(ThemeRole::Primary));
        self.hint_color = theme.color(ThemeRole::Muted);
        self.text_color = theme.color(ThemeRole::Foreground);
        self.muted_color = theme.color(ThemeRole::Muted);
        self.selected_fg = theme.color(ThemeRole::Foreground);
        self.selected_bg = Some(theme.color(ThemeRole::Highlight));
        self.disabled_color = theme.color(ThemeRole::Muted);
        self
    }

    pub fn set_y_offset(&mut self, y: u16) {
        self.y_offset = y;
    }

    pub fn tabs_value(&self) -> &[TabbedMenuTab] {
        &self.tabs
    }

    pub fn active_tab_value(&self) -> usize {
        self.normalized_active_tab()
    }

    pub fn selected_index(&self) -> usize {
        self.normalized_selected()
    }

    pub fn active_items(&self) -> &[TabbedMenuItem] {
        self.current_tab()
            .map(TabbedMenuTab::items_value)
            .unwrap_or(&[])
    }

    pub fn selected_item(&self) -> Option<&TabbedMenuItem> {
        self.active_items().get(self.normalized_selected())
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<TabbedMenuPanelMsg> {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => self.move_tab_left(),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => self.move_tab_right(),
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.normalized_selected().saturating_sub(1);
                self.keep_selected_visible(1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let selected = self.normalized_selected();
                if selected.saturating_add(1) < self.active_items().len() {
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
                    .min(self.active_items().len().saturating_sub(1));
                self.keep_selected_visible(step);
                None
            }
            KeyCode::Home => {
                self.selected = 0;
                self.scroll = 0;
                None
            }
            KeyCode::End => {
                self.selected = self.active_items().len().saturating_sub(1);
                self.keep_selected_visible(self.max_items.unwrap_or(10));
                None
            }
            KeyCode::Enter => self.selected_msg(),
            KeyCode::Esc => Some(TabbedMenuPanelMsg::Cancelled),
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<TabbedMenuPanelMsg> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let local_row = super::relative_mouse_row(mouse.row, self.y_offset)?;
                if self.should_show_tabs() && local_row == self.tab_row() {
                    let tab = self.tab_index_at_column(mouse.column)?;
                    if tab != self.normalized_active_tab() {
                        self.active_tab = tab;
                        self.selected = 0;
                        self.scroll = 0;
                    }
                    return Some(TabbedMenuPanelMsg::TabChanged(tab));
                }
                let item_row = local_row.checked_sub(self.item_start_row())?;
                let item_count = self.visible_item_count_for_height(usize::MAX);
                if item_row >= item_count {
                    return None;
                }
                let index = self.window_start(item_count).saturating_add(item_row);
                if index < self.active_items().len() {
                    self.selected = index;
                    self.selected_msg()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 || self.tabs.is_empty() {
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
        let Some(active_tab) = self.current_tab() else {
            return Element::Box(BoxElement::new().direction(FlexDirection::Column));
        };

        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            children.push(Element::Text(
                TextElement::new(title)
                    .fg(self.title_color.unwrap_or(active_tab.color))
                    .bold(),
            ));
        }

        if self.should_show_tabs() {
            children.push(self.tabs_element());
        }

        if let Some(hint) = self.hint.as_deref().filter(|hint| !hint.is_empty()) {
            children.push(Element::Text(TextElement::new(hint).fg(self.hint_color)));
        }

        let selected = self.normalized_selected();
        for index in self.element_item_range() {
            children.push(Element::Text(
                self.item_text_element(index, active_tab, selected),
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

    pub fn element_with_height<Msg>(&self, height: usize) -> Element<Msg> {
        let mut children = Vec::new();
        let Some(active_tab) = self.current_tab() else {
            return Element::Box(BoxElement::new().direction(FlexDirection::Column));
        };

        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            children.push(Element::Text(
                TextElement::new(title)
                    .fg(self.title_color.unwrap_or(active_tab.color))
                    .bold(),
            ));
        }

        if self.should_show_tabs() {
            children.push(self.tabs_element());
        }

        if let Some(hint) = self.hint.as_deref().filter(|hint| !hint.is_empty()) {
            children.push(Element::Text(TextElement::new(hint).fg(self.hint_color)));
        }

        let items = self.active_items();
        if items.is_empty() {
            if children.len() < height {
                let text = active_tab.empty_text.as_deref().unwrap_or("no items");
                children.push(Element::Text(TextElement::new(text).fg(self.muted_color)));
            }
        } else {
            let visible_items = self.visible_item_count_for_height(height);
            let start = self.window_start(visible_items);
            let end = start.saturating_add(visible_items).min(items.len());
            let selected = self.normalized_selected();
            for index in start..end {
                children.push(Element::Text(
                    self.item_text_element(index, active_tab, selected),
                ));
            }

            if self.show_scroll && items.len() > visible_items && visible_items > 0 {
                children.push(Element::Text(self.scroll_footer_element(
                    start,
                    end,
                    items.len(),
                )));
            }
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
        let Some(active_tab) = self.current_tab() else {
            return lines;
        };

        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            lines.push(
                Style::new()
                    .fg(self.title_color.unwrap_or(active_tab.color))
                    .bold()
                    .render(&fit_visible(
                        &format!("{}{}", " ".repeat(self.indent_for_width(width)), title),
                        width,
                    )),
            );
        }

        if self.should_show_tabs() {
            lines.push(self.render_tabs(width));
        }

        if let Some(hint) = self.hint.as_deref().filter(|hint| !hint.is_empty()) {
            lines.push(Style::new().fg(self.hint_color).render(&fit_visible(
                &format!("{}{}", " ".repeat(self.indent_for_width(width)), hint),
                width,
            )));
        }

        let items = self.active_items();
        if items.is_empty() {
            if lines.len() < height {
                let text = active_tab.empty_text.as_deref().unwrap_or("no items");
                lines.push(Style::new().fg(self.muted_color).render(&fit_visible(
                    &format!("{}{}", " ".repeat(self.indent_for_width(width)), text),
                    width,
                )));
            }
        } else {
            let visible_items = self.visible_item_count_for_height(height);
            let start = self.window_start(visible_items);
            let end = (start + visible_items).min(items.len());
            for index in start..end {
                lines.push(self.render_item(index, width, active_tab));
            }

            if self.show_scroll && items.len() > visible_items && visible_items > 0 {
                lines.push(self.render_scroll_footer(start, end, width, items.len()));
            }
        }

        if let Some(footer) = self.footer.as_deref().filter(|footer| !footer.is_empty()) {
            lines.push(Style::new().fg(self.muted_color).render(&fit_visible(
                &format!("{}{}", " ".repeat(self.indent_for_width(width)), footer),
                width,
            )));
        }

        lines
    }

    fn render_tabs(&self, width: usize) -> String {
        self.tab_strip(self.indent_for_width(width))
            .view(width as u16)
    }

    fn tabs_element<Msg>(&self) -> Element<Msg> {
        self.tab_strip(self.indent_for_element()).element()
    }

    fn tab_strip(&self, margin: usize) -> ChipStrip {
        let active = self.normalized_active_tab();
        let chips = self
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let chip = Chip::new(tab.label.clone());
                if index == active || self.inactive_tabs_use_tab_color {
                    chip.color(tab.color)
                } else {
                    chip
                }
            })
            .collect::<Vec<_>>();
        let active_bg = self
            .tabs
            .get(active)
            .map_or(self.muted_color, |tab| tab.color);

        ChipStrip::new(chips)
            .active(active)
            .margin(margin)
            .gap(self.tab_gap.min(MAX_TABBED_MENU_PANEL_TAB_GAP))
            .active_colors(
                self.active_tab_foreground.unwrap_or(self.selected_fg),
                active_bg,
            )
            .inactive_color(self.muted_color)
    }

    fn item_text_element(
        &self,
        index: usize,
        active_tab: &TabbedMenuTab,
        selected: usize,
    ) -> TextElement {
        let item = &active_tab.items[index];
        let mut text = TextElement::new(self.plain_item_line(index, None));
        if index == selected {
            text = text
                .fg(self.selected_fg)
                .bg(self.selected_bg.unwrap_or(active_tab.color))
                .bold();
        } else if item.disabled {
            text = text.fg(self.disabled_color);
        } else {
            text = text.fg(self.item_color(item, active_tab));
        }
        text
    }

    fn render_item(&self, index: usize, width: usize, active_tab: &TabbedMenuTab) -> String {
        let raw = fit_visible(&self.plain_item_line(index, Some(width)), width);
        let item = &active_tab.items[index];
        if index == self.normalized_selected() {
            Style::new()
                .fg(self.selected_fg)
                .bg(self.selected_bg.unwrap_or(active_tab.color))
                .render(&raw)
        } else if item.disabled {
            Style::new().fg(self.disabled_color).render(&raw)
        } else {
            Style::new()
                .fg(self.item_color(item, active_tab))
                .render(&raw)
        }
    }

    fn plain_item_line(&self, index: usize, width: Option<usize>) -> String {
        let Some(item) = self.active_items().get(index) else {
            return String::new();
        };
        let prefix = match width {
            Some(width) => self.item_prefix_for_width(item, width),
            None => self.item_prefix_for_element(item),
        };
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

    fn render_scroll_footer(&self, start: usize, end: usize, width: usize, total: usize) -> String {
        let up = if start > 0 { "↑" } else { " " };
        let down = if end < total { "↓" } else { " " };
        Style::new().fg(self.muted_color).render(&fit_visible(
            &format!(
                "{}{up}{down} {}/{}",
                " ".repeat(self.indent_for_width(width)),
                self.normalized_selected().saturating_add(1).min(total),
                total
            ),
            width,
        ))
    }

    fn scroll_footer_element(&self, start: usize, end: usize, total: usize) -> TextElement {
        let up = if start > 0 { "↑" } else { " " };
        let down = if end < total { "↓" } else { " " };
        TextElement::new(format!(
            "{}{up}{down} {}/{}",
            " ".repeat(self.indent_for_element()),
            self.normalized_selected().saturating_add(1).min(total),
            total
        ))
        .fg(self.muted_color)
    }

    fn move_tab_left(&mut self) -> Option<TabbedMenuPanelMsg> {
        let active = self.normalized_active_tab();
        if active == 0 {
            self.active_tab = active;
            return None;
        }
        self.active_tab = active - 1;
        self.selected = 0;
        self.scroll = 0;
        Some(TabbedMenuPanelMsg::TabChanged(self.active_tab))
    }

    fn move_tab_right(&mut self) -> Option<TabbedMenuPanelMsg> {
        let active = self.normalized_active_tab();
        if active.saturating_add(1) >= self.tabs.len() {
            self.active_tab = active;
            return None;
        }
        self.active_tab = active + 1;
        self.selected = 0;
        self.scroll = 0;
        Some(TabbedMenuPanelMsg::TabChanged(self.active_tab))
    }

    fn selected_msg(&self) -> Option<TabbedMenuPanelMsg> {
        let active_tab = self.normalized_active_tab();
        let selected = self.normalized_selected();
        let item = self.active_items().get(selected)?;
        if item.disabled {
            None
        } else {
            Some(TabbedMenuPanelMsg::Selected {
                tab: active_tab,
                item: selected,
            })
        }
    }

    fn visible_item_count_for_height(&self, height: usize) -> usize {
        if self.active_items().is_empty() {
            return 0;
        }
        let reserved = self.item_start_row()
            + usize::from(
                self.footer
                    .as_ref()
                    .is_some_and(|footer| !footer.is_empty()),
            )
            + usize::from(self.show_scroll && self.active_items().len() > 1);
        let available = height.saturating_sub(reserved).max(1);
        self.max_items.unwrap_or(available).min(available)
    }

    fn window_start(&self, visible_items: usize) -> usize {
        let item_count = self.active_items().len();
        if visible_items == 0 || item_count <= visible_items {
            return 0;
        }
        let max_start = item_count.saturating_sub(visible_items);
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
        let item_count = self.active_items().len();
        let visible_items = self.max_items.unwrap_or(item_count).min(item_count);
        let start = self.window_start(visible_items);
        let end = start.saturating_add(visible_items).min(item_count);
        start..end
    }

    fn keep_selected_visible(&mut self, window_hint: usize) {
        let visible_items = self.max_items.unwrap_or(window_hint.max(1));
        self.scroll = self.window_start(visible_items);
    }

    fn item_start_row(&self) -> usize {
        usize::from(self.title.as_ref().is_some_and(|title| !title.is_empty()))
            + usize::from(self.should_show_tabs())
            + usize::from(self.hint.as_ref().is_some_and(|hint| !hint.is_empty()))
    }

    fn tab_row(&self) -> usize {
        usize::from(self.title.as_ref().is_some_and(|title| !title.is_empty()))
    }

    fn tab_index_at_column(&self, column: u16) -> Option<usize> {
        let click_x = usize::from(column);
        let mut x = self.indent_for_element();
        let gap = self.tab_gap.min(MAX_TABBED_MENU_PANEL_TAB_GAP);
        for (index, tab) in self.tabs.iter().enumerate() {
            let tab_width = visible_len(tab.label()) + 2;
            if click_x >= x && click_x < x.saturating_add(tab_width) {
                return Some(index);
            }
            x = x.saturating_add(tab_width).saturating_add(gap);
        }
        None
    }

    fn should_show_tabs(&self) -> bool {
        self.tabs.len() > 1 || (self.show_tabs_when_single && !self.tabs.is_empty())
    }

    fn current_tab(&self) -> Option<&TabbedMenuTab> {
        self.tabs.get(self.normalized_active_tab())
    }

    fn item_color(&self, item: &TabbedMenuItem, active_tab: &TabbedMenuTab) -> Color {
        item.color.unwrap_or(if self.items_use_tab_color {
            active_tab.color
        } else {
            self.text_color
        })
    }

    fn clamped(mut self) -> Self {
        self.clamp_state();
        self
    }

    fn clamp_state(&mut self) {
        self.active_tab = self.normalized_active_tab();
        self.selected = self.normalized_selected();
    }

    fn normalized_active_tab(&self) -> usize {
        self.active_tab.min(self.tabs.len().saturating_sub(1))
    }

    fn normalized_selected(&self) -> usize {
        self.selected
            .min(self.active_items().len().saturating_sub(1))
    }

    fn indent_for_width(&self, width: usize) -> usize {
        self.indent.min(width).min(MAX_TABBED_MENU_PANEL_INDENT)
    }

    fn item_prefix_for_width(&self, item: &TabbedMenuItem, width: usize) -> String {
        let tail = truncate_visible(&self.item_prefix_tail(item), width);
        let tail_width = visible_len(&tail);
        let indent = self.indent.min(width.saturating_sub(tail_width));
        format!("{}{}", " ".repeat(indent), tail)
    }

    fn item_prefix_for_element(&self, item: &TabbedMenuItem) -> String {
        format!(
            "{}{}",
            " ".repeat(self.indent_for_element()),
            self.item_prefix_tail(item)
        )
    }

    fn item_prefix_tail(&self, item: &TabbedMenuItem) -> String {
        item.prefix
            .as_deref()
            .filter(|prefix| !prefix.is_empty())
            .map(|prefix| format!("{prefix} "))
            .unwrap_or_default()
    }

    fn indent_for_element(&self) -> usize {
        self.indent.min(MAX_TABBED_MENU_PANEL_INDENT)
    }
}

impl Selectable for TabbedMenuPanel {
    fn item_count(&self) -> usize {
        self.active_items().len()
    }

    fn selected_index(&self) -> Option<usize> {
        (!self.active_items().is_empty()).then(|| self.normalized_selected())
    }

    fn select_index(&mut self, index: usize) {
        self.selected = index;
        self.clamp_state();
    }
}

impl Scrollable for TabbedMenuPanel {
    fn scroll_offset(&self) -> usize {
        self.scroll
    }

    fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll = offset.min(self.active_items().len().saturating_sub(1));
    }
}

impl Tabbed for TabbedMenuPanel {
    fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    fn active_tab_index(&self) -> Option<usize> {
        (!self.tabs.is_empty()).then(|| self.normalized_active_tab())
    }

    fn set_active_tab_index(&mut self, index: usize) {
        self.active_tab = index;
        self.clamp_state();
    }
}

impl Activatable for TabbedMenuPanel {
    fn is_item_disabled(&self, index: usize) -> bool {
        self.active_items()
            .get(index)
            .is_some_and(TabbedMenuItem::is_disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};
    use crossterm::event::KeyModifiers;

    fn sample() -> TabbedMenuPanel {
        TabbedMenuPanel::new(vec![
            TabbedMenuTab::new("a3s-code", Color::Cyan).items(vec![
                TabbedMenuItem::new("openai/gpt-5").prefix("●"),
                TabbedMenuItem::new("openai/gpt-5-mini"),
            ]),
            TabbedMenuTab::new("Codex", Color::Rgb(115, 218, 202)).items(vec![
                TabbedMenuItem::new("gpt-5-codex").description("local login"),
                TabbedMenuItem::new("gpt-5-codex-fast").disabled(true),
            ]),
        ])
        .title("Select model")
        .hint("↑/↓ model · ←/→ account · Enter · Esc")
        .active_tab(1)
        .selected(0)
        .footer("2 sources")
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn with_theme_applies_semantic_colors() {
        let theme = Theme::tokyo_night();
        let panel = sample().with_theme(&theme);

        assert_eq!(panel.title_color, Some(theme.color(ThemeRole::Primary)));
        assert_eq!(panel.hint_color, theme.color(ThemeRole::Muted));
        assert_eq!(panel.text_color, theme.color(ThemeRole::Foreground));
        assert_eq!(panel.selected_bg, Some(theme.color(ThemeRole::Highlight)));
        assert_eq!(panel.disabled_color, theme.color(ThemeRole::Muted));
        assert_eq!(panel.tabs[0].color_value(), Color::Cyan);
    }

    #[test]
    fn themed_string_tabs_use_selected_and_muted_semantic_colors() {
        let theme = Theme::tokyo_night();
        let rendered = sample().with_theme(&theme).view(64, 8);
        let active_background = sample().tabs[1].color_value();

        assert!(rendered.contains(
            &Style::new()
                .fg(theme.color(ThemeRole::Muted))
                .render(" a3s-code ")
        ));
        assert!(rendered.contains(
            &Style::new()
                .fg(theme.color(ThemeRole::Foreground))
                .bg(active_background)
                .bold()
                .render(" Codex ")
        ));
        assert!(!rendered.contains(&Style::new().fg(Color::Cyan).render(" a3s-code ")));
    }

    #[test]
    fn themed_element_tabs_use_selected_and_muted_semantic_colors() {
        let theme = Theme::tokyo_night();
        let panel = sample().with_theme(&theme);
        let active_background = panel.tabs[1].color_value();
        let Element::Box(column) = panel.element::<()>() else {
            panic!("expected panel column");
        };
        let Element::Box(tabs) = &column.children[1] else {
            panic!("expected tab strip");
        };
        let Element::Text(inactive) = &tabs.children[1] else {
            panic!("expected inactive tab");
        };
        let Element::Text(active) = &tabs.children[3] else {
            panic!("expected active tab");
        };

        assert_eq!(inactive.content, " a3s-code ");
        assert_eq!(inactive.style.fg, Some(theme.color(ThemeRole::Muted)));
        assert_eq!(inactive.style.bg, None);
        assert_eq!(active.content, " Codex ");
        assert_eq!(active.style.fg, Some(theme.color(ThemeRole::Foreground)));
        assert_eq!(active.style.bg, Some(active_background));
        assert!(active.style.bold);
    }

    #[test]
    fn inactive_tabs_can_use_their_configured_colors() {
        let panel = sample().inactive_tabs_use_tab_color(true);
        let rendered = panel.view(64, 8);

        assert!(rendered.contains(&Style::new().fg(Color::Cyan).render(" a3s-code ")));

        let Element::Box(column) = panel.element::<()>() else {
            panic!("expected panel column");
        };
        let Element::Box(tabs) = &column.children[1] else {
            panic!("expected tab strip");
        };
        let Element::Text(inactive) = &tabs.children[1] else {
            panic!("expected inactive tab");
        };

        assert_eq!(inactive.style.fg, Some(Color::Cyan));
    }

    #[test]
    fn active_tab_foreground_is_independent_from_selected_item_foreground() {
        let panel = sample()
            .selected_colors(Color::White, Color::Blue)
            .active_tab_foreground(Color::Black);

        let Element::Box(column) = panel.element::<()>() else {
            panic!("expected panel column");
        };
        let Element::Box(tabs) = &column.children[1] else {
            panic!("expected tab strip");
        };
        let Element::Text(active_tab) = &tabs.children[3] else {
            panic!("expected active tab");
        };
        let Element::Text(selected_item) = &column.children[3] else {
            panic!("expected selected item");
        };

        assert_eq!(active_tab.style.fg, Some(Color::Black));
        assert_eq!(selected_item.style.fg, Some(Color::White));
        assert_eq!(selected_item.style.bg, Some(Color::Blue));
    }

    #[test]
    fn renders_tabs_hint_items_and_footer() {
        let rendered = sample().fill_height(true).view(64, 8);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("Select model"));
        assert!(plain.contains("a3s-code"));
        assert!(plain.contains("Codex"));
        assert!(plain.contains("gpt-5-codex"));
        assert!(plain.contains("2 sources"));
        assert_eq!(rendered.lines().count(), 8);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 64, "{line:?}");
        }
    }

    #[test]
    fn scroll_keeps_selected_visible() {
        let items = (0..20)
            .map(|idx| TabbedMenuItem::new(format!("session-{idx}")))
            .collect::<Vec<_>>();
        let rendered = TabbedMenuPanel::new(vec![
            TabbedMenuTab::new("Claude", Color::Yellow).items(items)
        ])
        .show_tabs_when_single(true)
        .selected(18)
        .max_items(4)
        .view(40, 6);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("session-18"), "{plain:?}");
        assert!(!plain.contains("session-0"), "{plain:?}");
    }

    #[test]
    fn empty_tab_renders_empty_state() {
        let rendered = TabbedMenuPanel::new(vec![
            TabbedMenuTab::new("Codex", Color::Cyan).empty_text("(no Codex sessions)")
        ])
        .show_tabs_when_single(true)
        .view(40, 4);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("Codex"));
        assert!(plain.contains("no Codex sessions"));
    }

    #[test]
    fn cjk_rows_fit_requested_width() {
        let rendered = TabbedMenuPanel::new(vec![TabbedMenuTab::new("OS 网关", Color::Magenta)
            .item(TabbedMenuItem::new("模型-中文-very-long-tail").description("可用"))])
        .show_tabs_when_single(true)
        .view(24, 4);

        assert!(strip_ansi(&rendered).contains("模型"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 24, "{line:?}");
        }
    }

    #[test]
    fn oversized_spacing_is_clamped_to_render_width() {
        let panel =
            TabbedMenuPanel::new(vec![TabbedMenuTab::new("Codex", Color::Cyan)
                .item(TabbedMenuItem::new("gpt-5").prefix("●"))])
            .title("Models")
            .hint("hint")
            .indent(usize::MAX)
            .tab_gap(usize::MAX)
            .show_tabs_when_single(true)
            .footer("footer")
            .fill_height(true);
        let rendered = panel.view(8, 5);
        let item = panel.active_items().first().unwrap();
        let prefix = panel.item_prefix_for_width(item, 8);
        let line = panel.plain_item_line(0, Some(8));

        assert_eq!(panel.indent, MAX_TABBED_MENU_PANEL_INDENT);
        assert_eq!(panel.tab_gap, MAX_TABBED_MENU_PANEL_TAB_GAP);
        assert_eq!(panel.indent_for_width(8), 8);
        assert_eq!(visible_len(&prefix), 8);
        assert_eq!(visible_len(&line), 8);
        assert!(rendered.lines().all(|line| visible_len(line) == 8));

        let Element::Box(column) = panel.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Text(item) = &column.children[3] else {
            panic!("expected item text");
        };
        assert_eq!(
            visible_len(&item.content),
            MAX_TABBED_MENU_PANEL_INDENT + visible_len("● gpt-5")
        );
    }

    #[test]
    fn oversized_item_limit_is_clamped() {
        let panel = TabbedMenuPanel::new(vec![TabbedMenuTab::new("Codex", Color::Cyan)
            .item(TabbedMenuItem::new("gpt-5"))
            .item(TabbedMenuItem::new("gpt-5-mini"))])
        .show_tabs_when_single(true)
        .max_items(usize::MAX);
        let rendered = panel.view(24, 4);

        assert_eq!(panel.max_items, Some(MAX_TABBED_MENU_PANEL_ITEMS));
        assert!(rendered.lines().all(|line| visible_len(line) == 24));
    }

    #[test]
    fn key_handling_switches_tabs_moves_and_selects() {
        let mut panel = sample();
        assert_eq!(panel.active_tab_value(), 1);
        assert_eq!(
            panel.handle_key(&key(KeyCode::Left)),
            Some(TabbedMenuPanelMsg::TabChanged(0))
        );
        assert_eq!(panel.active_tab_value(), 0);
        assert_eq!(panel.selected_index(), 0);
        assert_eq!(panel.handle_key(&key(KeyCode::Down)), None);
        assert_eq!(panel.selected_index(), 1);
        assert_eq!(
            panel.handle_key(&key(KeyCode::Enter)),
            Some(TabbedMenuPanelMsg::Selected { tab: 0, item: 1 })
        );
        assert_eq!(
            panel.handle_key(&key(KeyCode::Esc)),
            Some(TabbedMenuPanelMsg::Cancelled)
        );
    }

    #[test]
    fn stale_active_tab_index_is_clamped_during_navigation() {
        let mut panel = sample();
        panel.active_tab = usize::MAX;

        assert_eq!(panel.handle_key(&key(KeyCode::Right)), None);
        assert_eq!(panel.active_tab_value(), 1);

        assert_eq!(
            panel.handle_key(&key(KeyCode::Left)),
            Some(TabbedMenuPanelMsg::TabChanged(0))
        );
        assert_eq!(panel.active_tab_value(), 0);
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn stale_selection_is_normalized_for_rendering_and_input() {
        let mut panel = TabbedMenuPanel::new(vec![TabbedMenuTab::new("Codex", Color::Cyan)
            .item(TabbedMenuItem::new("one"))
            .item(TabbedMenuItem::new("two"))])
        .show_tabs_when_single(true)
        .max_items(1);
        panel.selected = usize::MAX;

        assert_eq!(panel.selected_index(), 1);
        assert_eq!(
            panel.selected_item().map(TabbedMenuItem::label),
            Some("two")
        );
        assert_eq!(
            panel.handle_key(&key(KeyCode::Enter)),
            Some(TabbedMenuPanelMsg::Selected { tab: 0, item: 1 })
        );

        let plain = strip_ansi(&panel.view(24, 4));
        assert!(plain.contains("two"), "{plain:?}");
        assert!(!plain.contains("one"), "{plain:?}");

        let Element::Box(column) = panel.element::<()>() else {
            panic!("expected column element");
        };
        assert_eq!(column.children.len(), 2);
        let Element::Text(selected_item) = &column.children[1] else {
            panic!("expected selected item");
        };
        assert_eq!(selected_item.content, "  two");

        assert_eq!(panel.handle_key(&key(KeyCode::Down)), None);
        assert_eq!(panel.selected_index(), 1);

        assert_eq!(panel.handle_key(&key(KeyCode::Up)), None);
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn huge_page_down_saturates_selection() {
        let mut panel = sample().active_tab(0).selected(1).max_items(usize::MAX);

        assert_eq!(panel.handle_key(&key(KeyCode::PageDown)), None);

        assert_eq!(panel.selected_index(), panel.active_items().len() - 1);
    }

    #[test]
    fn disabled_item_does_not_select() {
        let mut panel = sample().selected(1);

        assert_eq!(panel.handle_key(&key(KeyCode::Enter)), None);
    }

    #[test]
    fn mouse_click_above_offset_is_ignored() {
        let mut panel = sample();
        panel.set_y_offset(4);

        let msg = panel.handle_mouse(&MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 3,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(msg, None);
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn mouse_click_on_tab_switches_active_tab() {
        let mut panel = sample();

        let msg = panel.handle_mouse(&MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 4,
            row: 1,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(msg, Some(TabbedMenuPanelMsg::TabChanged(0)));
        assert_eq!(panel.active_tab_value(), 0);
        assert_eq!(panel.selected_index(), 0);
    }

    #[test]
    fn mouse_click_between_tabs_is_ignored() {
        let mut panel = sample();

        let msg = panel.handle_mouse(&MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 12,
            row: 1,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(msg, None);
        assert_eq!(panel.active_tab_value(), 1);
    }

    #[test]
    fn zero_size_or_empty_tabs_renders_empty_string() {
        assert_eq!(sample().view(0, 8), "");
        assert_eq!(sample().view(40, 0), "");
        assert_eq!(TabbedMenuPanel::new(Vec::new()).view(40, 4), "");
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
    fn element_respects_max_items_window() {
        let items = (0..5)
            .map(|idx| TabbedMenuItem::new(format!("session-{idx}")))
            .collect::<Vec<_>>();
        let el: Element<()> =
            TabbedMenuPanel::new(vec![TabbedMenuTab::new("Codex", Color::Cyan).items(items)])
                .show_tabs_when_single(true)
                .selected(3)
                .scroll(1)
                .max_items(3)
                .element();

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        let text = column
            .children
            .iter()
            .filter_map(Element::text_content)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(text.contains("session-1"));
        assert!(text.contains("session-2"));
        assert!(text.contains("session-3"));
        assert!(!text.contains("session-0"));
        assert!(!text.contains("session-4"));
    }

    #[test]
    fn element_with_height_zero_returns_empty_column() {
        let el: Element<()> = sample().element_with_height(0);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        assert!(column.children.is_empty());
    }

    #[test]
    fn element_with_height_limits_items_and_keeps_scroll_footer() {
        let items = (0..6)
            .map(|idx| TabbedMenuItem::new(format!("session-{idx}")))
            .collect::<Vec<_>>();
        let el: Element<()> =
            TabbedMenuPanel::new(vec![TabbedMenuTab::new("Codex", Color::Cyan).items(items)])
                .show_tabs_when_single(true)
                .selected(4)
                .scroll(2)
                .element_with_height(4);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        let text = column
            .children
            .iter()
            .filter_map(Element::text_content)
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(column.children.len(), 4);
        assert!(text.contains("session-3"), "{text:?}");
        assert!(text.contains("session-4"), "{text:?}");
        assert!(text.contains("5/6"), "{text:?}");
        assert!(!text.contains("session-0"), "{text:?}");
    }

    #[test]
    fn element_with_height_empty_tab_renders_empty_state_and_padding() {
        let el: Element<()> = TabbedMenuPanel::new(vec![
            TabbedMenuTab::new("Codex", Color::Cyan).empty_text("(no models)")
        ])
        .show_tabs_when_single(true)
        .fill_height(true)
        .element_with_height(4);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        let text = column
            .children
            .iter()
            .filter_map(Element::text_content)
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(column.children.len(), 4);
        assert!(text.contains("(no models)"), "{text:?}");
    }
}
