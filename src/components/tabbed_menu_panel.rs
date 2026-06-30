use super::chip_strip::{Chip, ChipStrip};
use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};
use crossterm::event::KeyCode;

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
/// This extracts the model/relay picker pattern from the CLI: colored source
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
        self.max_items = Some(max_items.max(1));
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
        self.indent = indent;
        self
    }

    pub fn tab_gap(mut self, gap: usize) -> Self {
        self.tab_gap = gap;
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

    pub fn set_y_offset(&mut self, y: u16) {
        self.y_offset = y;
    }

    pub fn tabs_value(&self) -> &[TabbedMenuTab] {
        &self.tabs
    }

    pub fn active_tab_value(&self) -> usize {
        self.active_tab
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn active_items(&self) -> &[TabbedMenuItem] {
        self.current_tab()
            .map(TabbedMenuTab::items_value)
            .unwrap_or(&[])
    }

    pub fn selected_item(&self) -> Option<&TabbedMenuItem> {
        self.active_items().get(self.selected)
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<TabbedMenuPanelMsg> {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => self.move_tab_left(),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => self.move_tab_right(),
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                self.keep_selected_visible(1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.active_items().len() {
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
                self.selected =
                    (self.selected + step).min(self.active_items().len().saturating_sub(1));
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
                let local_row = mouse.row.saturating_sub(self.y_offset) as usize;
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

        for (index, item) in self.active_items().iter().enumerate() {
            let mut text = TextElement::new(self.plain_item_line(index, None));
            if index == self.selected {
                text = text
                    .fg(self.selected_fg)
                    .bg(self.selected_bg.unwrap_or(active_tab.color))
                    .bold();
            } else if item.disabled {
                text = text.fg(self.disabled_color);
            } else {
                text = text.fg(self.item_color(item, active_tab));
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
        let Some(active_tab) = self.current_tab() else {
            return lines;
        };

        if let Some(title) = self.title.as_deref().filter(|title| !title.is_empty()) {
            lines.push(
                Style::new()
                    .fg(self.title_color.unwrap_or(active_tab.color))
                    .bold()
                    .render(&fit_visible(
                        &format!("{}{}", " ".repeat(self.indent), title),
                        width,
                    )),
            );
        }

        if self.should_show_tabs() {
            lines.push(self.render_tabs(width));
        }

        if let Some(hint) = self.hint.as_deref().filter(|hint| !hint.is_empty()) {
            lines.push(Style::new().fg(self.hint_color).render(&fit_visible(
                &format!("{}{}", " ".repeat(self.indent), hint),
                width,
            )));
        }

        let items = self.active_items();
        if items.is_empty() {
            if lines.len() < height {
                let text = active_tab.empty_text.as_deref().unwrap_or("no items");
                lines.push(Style::new().fg(self.muted_color).render(&fit_visible(
                    &format!("{}{}", " ".repeat(self.indent), text),
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
                &format!("{}{}", " ".repeat(self.indent), footer),
                width,
            )));
        }

        lines
    }

    fn render_tabs(&self, width: usize) -> String {
        let chips = self
            .tabs
            .iter()
            .map(|tab| Chip::new(tab.label.clone()).color(tab.color))
            .collect::<Vec<_>>();
        ChipStrip::new(chips)
            .active(self.active_tab)
            .margin(self.indent)
            .gap(self.tab_gap)
            .view(width as u16)
    }

    fn tabs_element<Msg>(&self) -> Element<Msg> {
        let chips = self
            .tabs
            .iter()
            .map(|tab| Chip::new(tab.label.clone()).color(tab.color))
            .collect::<Vec<_>>();
        ChipStrip::new(chips)
            .active(self.active_tab)
            .margin(self.indent)
            .gap(self.tab_gap)
            .element()
    }

    fn render_item(&self, index: usize, width: usize, active_tab: &TabbedMenuTab) -> String {
        let raw = fit_visible(&self.plain_item_line(index, Some(width)), width);
        let item = &active_tab.items[index];
        if index == self.selected {
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
        let mut prefix = " ".repeat(self.indent);
        if let Some(marker) = item.prefix.as_deref().filter(|prefix| !prefix.is_empty()) {
            prefix.push_str(marker);
            prefix.push(' ');
        }
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
                " ".repeat(self.indent),
                self.selected.saturating_add(1).min(total),
                total
            ),
            width,
        ))
    }

    fn move_tab_left(&mut self) -> Option<TabbedMenuPanelMsg> {
        if self.active_tab == 0 {
            return None;
        }
        self.active_tab -= 1;
        self.selected = 0;
        self.scroll = 0;
        Some(TabbedMenuPanelMsg::TabChanged(self.active_tab))
    }

    fn move_tab_right(&mut self) -> Option<TabbedMenuPanelMsg> {
        if self.active_tab + 1 >= self.tabs.len() {
            return None;
        }
        self.active_tab += 1;
        self.selected = 0;
        self.scroll = 0;
        Some(TabbedMenuPanelMsg::TabChanged(self.active_tab))
    }

    fn selected_msg(&self) -> Option<TabbedMenuPanelMsg> {
        let item = self.active_items().get(self.selected)?;
        if item.disabled {
            None
        } else {
            Some(TabbedMenuPanelMsg::Selected {
                tab: self.active_tab,
                item: self.selected,
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
            + usize::from(self.should_show_tabs())
            + usize::from(self.hint.as_ref().is_some_and(|hint| !hint.is_empty()))
    }

    fn should_show_tabs(&self) -> bool {
        self.tabs.len() > 1 || (self.show_tabs_when_single && !self.tabs.is_empty())
    }

    fn current_tab(&self) -> Option<&TabbedMenuTab> {
        self.tabs.get(self.active_tab)
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
        self.active_tab = self.active_tab.min(self.tabs.len().saturating_sub(1));
        self.selected = self
            .selected
            .min(self.active_items().len().saturating_sub(1));
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
    fn disabled_item_does_not_select() {
        let mut panel = sample().selected(1);

        assert_eq!(panel.handle_key(&key(KeyCode::Enter)), None);
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
}
