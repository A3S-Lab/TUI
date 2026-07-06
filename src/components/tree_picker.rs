use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::interaction::{Activatable, Scrollable, Selectable};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};
use crate::theme::{Theme, ThemeRole};
use crossterm::event::KeyCode;

const MAX_TREE_PICKER_DEPTH_INDENT: usize = u16::MAX as usize;
const MAX_TREE_PICKER_DEPTH_WIDTH: usize = u16::MAX as usize;
const MAX_TREE_PICKER_INDENT: usize = u16::MAX as usize;
const MAX_TREE_PICKER_ITEM_DEPTH: usize = u16::MAX as usize;
const MAX_TREE_PICKER_ITEMS: usize = u16::MAX as usize;

/// Node kind for a [`TreePickerItem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreePickerItemKind {
    Branch { open: bool },
    Leaf,
}

/// One visible row in a [`TreePicker`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreePickerItem {
    label: String,
    description: Option<String>,
    depth: usize,
    kind: TreePickerItemKind,
    color: Option<Color>,
    disabled: bool,
}

impl TreePickerItem {
    pub fn branch(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            depth: 0,
            kind: TreePickerItemKind::Branch { open: false },
            color: None,
            disabled: false,
        }
    }

    pub fn leaf(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            depth: 0,
            kind: TreePickerItemKind::Leaf,
            color: None,
            disabled: false,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth.min(MAX_TREE_PICKER_ITEM_DEPTH);
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        if matches!(self.kind, TreePickerItemKind::Branch { .. }) {
            self.kind = TreePickerItemKind::Branch { open };
        }
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

    pub fn depth_value(&self) -> usize {
        self.depth
    }

    pub fn kind_value(&self) -> TreePickerItemKind {
        self.kind
    }

    pub fn is_branch(&self) -> bool {
        matches!(self.kind, TreePickerItemKind::Branch { .. })
    }

    pub fn is_open(&self) -> bool {
        matches!(self.kind, TreePickerItemKind::Branch { open: true })
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// Message returned by [`TreePicker`] input handlers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreePickerMsg {
    Selected(usize),
    Toggled(usize),
    Opened(usize),
    Closed(usize),
    Cancelled,
}

/// Scrollable selectable tree for file pickers and hierarchy palettes.
///
/// Unlike [`Tree`](super::Tree), this component renders a flattened list of
/// visible rows. The caller owns expansion state and can rebuild the row list
/// after receiving open, close, or toggle messages.
#[derive(Debug, Clone)]
pub struct TreePicker {
    title: Option<String>,
    subtitle: Option<String>,
    items: Vec<TreePickerItem>,
    selected: usize,
    scroll: usize,
    max_items: Option<usize>,
    show_scroll: bool,
    fill_height: bool,
    y_offset: u16,
    indent: usize,
    depth_indent: usize,
    open_marker: String,
    closed_marker: String,
    leaf_marker: String,
    title_color: Color,
    subtitle_color: Color,
    branch_color: Color,
    leaf_color: Color,
    muted_color: Color,
    selected_fg: Color,
    selected_bg: Color,
    disabled_color: Color,
    footer: Option<String>,
}

impl TreePicker {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            subtitle: None,
            items: Vec::new(),
            selected: 0,
            scroll: 0,
            max_items: None,
            show_scroll: true,
            fill_height: false,
            y_offset: 0,
            indent: 2,
            depth_indent: 2,
            open_marker: "▾".to_string(),
            closed_marker: "▸".to_string(),
            leaf_marker: " ".to_string(),
            title_color: Color::Cyan,
            subtitle_color: Color::BrightBlack,
            branch_color: Color::Cyan,
            leaf_color: Color::White,
            muted_color: Color::BrightBlack,
            selected_fg: Color::BrightWhite,
            selected_bg: Color::Cyan,
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

    pub fn item(mut self, item: TreePickerItem) -> Self {
        self.items.push(item);
        self.clamp_selection();
        self
    }

    pub fn items(mut self, items: Vec<TreePickerItem>) -> Self {
        self.items = items;
        self.clamp_selection();
        self
    }

    pub fn add_item(&mut self, item: TreePickerItem) {
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
        self.max_items = Some(max_items.clamp(1, MAX_TREE_PICKER_ITEMS));
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
        self.indent = indent.min(MAX_TREE_PICKER_INDENT);
        self
    }

    pub fn depth_indent(mut self, indent: usize) -> Self {
        self.depth_indent = indent.clamp(1, MAX_TREE_PICKER_DEPTH_INDENT);
        self
    }

    pub fn markers(
        mut self,
        open: impl Into<String>,
        closed: impl Into<String>,
        leaf: impl Into<String>,
    ) -> Self {
        let open = open.into();
        let closed = closed.into();
        let leaf = leaf.into();
        if !open.is_empty() {
            self.open_marker = open;
        }
        if !closed.is_empty() {
            self.closed_marker = closed;
        }
        if !leaf.is_empty() {
            self.leaf_marker = leaf;
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

    pub fn branch_color(mut self, color: Color) -> Self {
        self.branch_color = color;
        self
    }

    pub fn leaf_color(mut self, color: Color) -> Self {
        self.leaf_color = color;
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

    /// Apply semantic colors from a theme while preserving content and layout.
    pub fn with_theme(mut self, theme: &Theme) -> Self {
        self.title_color = theme.color(ThemeRole::Primary);
        self.subtitle_color = theme.color(ThemeRole::Muted);
        self.branch_color = theme.color(ThemeRole::Primary);
        self.leaf_color = theme.color(ThemeRole::Foreground);
        self.muted_color = theme.color(ThemeRole::Muted);
        self.selected_fg = theme.color(ThemeRole::Foreground);
        self.selected_bg = theme.color(ThemeRole::Highlight);
        self.disabled_color = theme.color(ThemeRole::Muted);
        self
    }

    pub fn set_y_offset(&mut self, y: u16) {
        self.y_offset = y;
    }

    pub fn items_value(&self) -> &[TreePickerItem] {
        &self.items
    }

    pub fn selected_index(&self) -> usize {
        self.normalized_selected()
    }

    pub fn selected_item(&self) -> Option<&TreePickerItem> {
        self.items.get(self.normalized_selected())
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<TreePickerMsg> {
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
            KeyCode::Right => self.branch_msg(TreePickerMsg::Opened, false),
            KeyCode::Left => self.branch_msg(TreePickerMsg::Closed, true),
            KeyCode::Enter => self.activate_selected(),
            KeyCode::Esc => Some(TreePickerMsg::Cancelled),
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<TreePickerMsg> {
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
                    self.activate_selected()
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
            text = text.fg(item.color.unwrap_or(self.default_item_color(item)));
        }
        text
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
            Style::new()
                .fg(item.color.unwrap_or(self.default_item_color(item)))
                .render(&raw)
        }
    }

    fn plain_item_line(&self, index: usize, width: Option<usize>) -> String {
        let Some(item) = self.items.get(index) else {
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

    fn activate_selected(&self) -> Option<TreePickerMsg> {
        let selected = self.normalized_selected();
        let item = self.items.get(selected)?;
        if item.disabled {
            return None;
        }
        if item.is_branch() {
            Some(TreePickerMsg::Toggled(selected))
        } else {
            Some(TreePickerMsg::Selected(selected))
        }
    }

    fn branch_msg(
        &self,
        f: impl FnOnce(usize) -> TreePickerMsg,
        expected_open: bool,
    ) -> Option<TreePickerMsg> {
        let selected = self.normalized_selected();
        let item = self.items.get(selected)?;
        if item.disabled || !item.is_branch() || item.is_open() != expected_open {
            None
        } else {
            Some(f(selected))
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

    fn default_item_color(&self, item: &TreePickerItem) -> Color {
        if item.is_branch() {
            self.branch_color
        } else {
            self.leaf_color
        }
    }

    fn indent_for_width(&self, width: usize) -> usize {
        self.indent.min(width).min(MAX_TREE_PICKER_INDENT)
    }

    fn item_prefix_for_width(&self, item: &TreePickerItem, width: usize) -> String {
        let tail = truncate_visible(&self.item_prefix_tail(item), width);
        let tail_width = visible_len(&tail);
        let indent = self.indent.min(width.saturating_sub(tail_width));
        let depth_width = self
            .item_depth_width_for_element(item)
            .min(width.saturating_sub(indent).saturating_sub(tail_width));
        format!("{}{}{}", " ".repeat(indent), " ".repeat(depth_width), tail)
    }

    fn item_prefix_for_element(&self, item: &TreePickerItem) -> String {
        format!(
            "{}{}{}",
            " ".repeat(self.indent_for_element()),
            " ".repeat(self.item_depth_width_for_element(item)),
            self.item_prefix_tail(item)
        )
    }

    fn item_prefix_tail(&self, item: &TreePickerItem) -> String {
        format!("{} ", self.item_marker(item))
    }

    fn item_marker(&self, item: &TreePickerItem) -> &str {
        match item.kind {
            TreePickerItemKind::Branch { open: true } => self.open_marker.as_str(),
            TreePickerItemKind::Branch { open: false } => self.closed_marker.as_str(),
            TreePickerItemKind::Leaf => self.leaf_marker.as_str(),
        }
    }

    fn item_depth_width_for_element(&self, item: &TreePickerItem) -> usize {
        item.depth
            .saturating_mul(self.depth_indent)
            .min(MAX_TREE_PICKER_DEPTH_WIDTH)
    }

    fn indent_for_element(&self) -> usize {
        self.indent.min(MAX_TREE_PICKER_INDENT)
    }
}

impl Default for TreePicker {
    fn default() -> Self {
        Self::without_title()
    }
}

impl Selectable for TreePicker {
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

impl Scrollable for TreePicker {
    fn scroll_offset(&self) -> usize {
        self.scroll
    }

    fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll = offset.min(self.items.len().saturating_sub(1));
    }
}

impl Activatable for TreePicker {
    fn is_item_disabled(&self, index: usize) -> bool {
        self.items
            .get(index)
            .is_some_and(TreePickerItem::is_disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};
    use crossterm::event::KeyModifiers;

    fn sample() -> TreePicker {
        TreePicker::new("@ file")
            .subtitle("↑/↓ · →/← folder · Enter · Esc")
            .items(vec![
                TreePickerItem::branch("src").open(true),
                TreePickerItem::leaf("main.rs").depth(1),
                TreePickerItem::leaf("lib.rs").depth(1),
                TreePickerItem::branch("tests").open(false),
                TreePickerItem::leaf("README.md"),
            ])
            .selected(1)
            .footer("5 files")
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
        let picker = TreePicker::without_title().with_theme(&theme);

        assert_eq!(picker.title_color, theme.color(ThemeRole::Primary));
        assert_eq!(picker.subtitle_color, theme.color(ThemeRole::Muted));
        assert_eq!(picker.branch_color, theme.color(ThemeRole::Primary));
        assert_eq!(picker.leaf_color, theme.color(ThemeRole::Foreground));
        assert_eq!(picker.selected_bg, theme.color(ThemeRole::Highlight));
        assert_eq!(picker.disabled_color, theme.color(ThemeRole::Muted));
    }

    #[test]
    fn renders_open_closed_and_leaf_rows() {
        let rendered = sample().view(48, 8);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("@ file"));
        assert!(plain.contains("▾ src"));
        assert!(plain.contains("  main.rs"));
        assert!(plain.contains("▸ tests"));
        assert!(plain.contains("5 files"));
        assert_eq!(rendered.lines().count(), 8);
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 48, "{line:?}");
        }
    }

    #[test]
    fn scroll_keeps_selected_item_visible() {
        let items = (0..20)
            .map(|idx| TreePickerItem::leaf(format!("file-{idx}.rs")))
            .collect::<Vec<_>>();
        let rendered = TreePicker::new("@ file")
            .items(items)
            .selected(18)
            .max_items(4)
            .view(32, 6);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("file-18.rs"), "{plain:?}");
        assert!(!plain.contains("file-0.rs"), "{plain:?}");
    }

    #[test]
    fn cjk_labels_fit_requested_width() {
        let rendered = TreePicker::new("@ file")
            .item(TreePickerItem::branch("源码目录").open(true))
            .item(
                TreePickerItem::leaf("组件测试文件-with-a-long-tail.rs")
                    .depth(1)
                    .description("modified"),
            )
            .selected(1)
            .view(24, 4);

        assert!(strip_ansi(&rendered).contains("组件"));
        for line in rendered.lines() {
            assert_eq!(visible_len(line), 24, "{line:?}");
        }
    }

    #[test]
    fn oversized_spacing_is_clamped_to_render_width() {
        let picker = TreePicker::new("@ file")
            .subtitle("hint")
            .indent(usize::MAX)
            .depth_indent(usize::MAX)
            .item(
                TreePickerItem::branch("src")
                    .depth(usize::MAX)
                    .description("dir"),
            )
            .footer("footer")
            .fill_height(true);
        let rendered = picker.view(8, 4);
        let item = picker.items.first().unwrap();
        let prefix = picker.item_prefix_for_width(item, 8);
        let line = picker.plain_item_line(0, Some(8));

        assert_eq!(picker.indent, MAX_TREE_PICKER_INDENT);
        assert_eq!(picker.depth_indent, MAX_TREE_PICKER_DEPTH_INDENT);
        assert_eq!(item.depth, MAX_TREE_PICKER_ITEM_DEPTH);
        assert_eq!(picker.indent_for_width(8), 8);
        assert_eq!(visible_len(&prefix), 8);
        assert_eq!(visible_len(&line), 8);
        assert!(rendered.lines().all(|line| visible_len(line) == 8));

        let Element::Box(column) = picker.element::<()>() else {
            panic!("expected column element");
        };
        let Element::Text(item) = &column.children[2] else {
            panic!("expected item text");
        };
        assert_eq!(
            visible_len(&item.content),
            MAX_TREE_PICKER_INDENT + MAX_TREE_PICKER_DEPTH_WIDTH + visible_len("▸ src  dir")
        );
    }

    #[test]
    fn oversized_item_limit_is_clamped() {
        let picker = TreePicker::new("@ file")
            .max_items(usize::MAX)
            .item(TreePickerItem::branch("src").open(true))
            .item(TreePickerItem::leaf("main.rs").depth(1));
        let rendered = picker.view(24, 4);

        assert_eq!(picker.max_items, Some(MAX_TREE_PICKER_ITEMS));
        assert!(rendered.lines().all(|line| visible_len(line) == 24));
    }

    #[test]
    fn key_handling_moves_and_emits_tree_actions() {
        let mut picker = sample();
        assert_eq!(picker.selected_index(), 1);

        assert_eq!(
            picker.handle_key(&key(KeyCode::Enter)),
            Some(TreePickerMsg::Selected(1))
        );
        assert_eq!(picker.handle_key(&key(KeyCode::Down)), None);
        assert_eq!(picker.handle_key(&key(KeyCode::Down)), None);
        assert_eq!(picker.selected_index(), 3);
        assert_eq!(
            picker.handle_key(&key(KeyCode::Right)),
            Some(TreePickerMsg::Opened(3))
        );
        assert_eq!(
            picker.handle_key(&key(KeyCode::Enter)),
            Some(TreePickerMsg::Toggled(3))
        );

        let mut opened = sample().selected(0);
        assert_eq!(
            opened.handle_key(&key(KeyCode::Left)),
            Some(TreePickerMsg::Closed(0))
        );
        assert_eq!(
            opened.handle_key(&key(KeyCode::Esc)),
            Some(TreePickerMsg::Cancelled)
        );
    }

    #[test]
    fn huge_page_down_saturates_selection() {
        let mut picker = sample().selected(1).max_items(usize::MAX);

        assert_eq!(picker.handle_key(&key(KeyCode::PageDown)), None);

        assert_eq!(picker.selected_index(), picker.items_value().len() - 1);
    }

    #[test]
    fn stale_selection_is_normalized_for_rendering_and_input() {
        let mut picker = TreePicker::without_title()
            .max_items(1)
            .item(TreePickerItem::leaf("one"))
            .item(TreePickerItem::leaf("two"));
        picker.selected = usize::MAX;

        assert_eq!(picker.selected_index(), 1);
        assert_eq!(
            picker.selected_item().map(TreePickerItem::label),
            Some("two")
        );
        assert_eq!(
            picker.handle_key(&key(KeyCode::Enter)),
            Some(TreePickerMsg::Selected(1))
        );

        let plain = strip_ansi(&picker.view(24, 2));
        assert!(plain.contains("two"), "{plain:?}");
        assert!(!plain.contains("one"), "{plain:?}");
        assert!(plain.contains("2/2"), "{plain:?}");

        let Element::Box(column) = picker.element::<()>() else {
            panic!("expected column element");
        };
        assert_eq!(column.children.len(), 1);
        let Element::Text(selected_item) = &column.children[0] else {
            panic!("expected selected item");
        };
        assert_eq!(selected_item.content, "    two");
        assert_eq!(selected_item.style.bg, Some(Color::Cyan));

        assert_eq!(picker.handle_key(&key(KeyCode::Down)), None);
        assert_eq!(picker.selected_index(), 1);

        assert_eq!(picker.handle_key(&key(KeyCode::Up)), None);
        assert_eq!(picker.selected_index(), 0);
    }

    #[test]
    fn disabled_items_do_not_emit_actions() {
        let mut picker = TreePicker::new("@ file")
            .item(TreePickerItem::branch("target").disabled(true))
            .item(TreePickerItem::leaf("main.rs"))
            .selected(0);

        assert_eq!(picker.handle_key(&key(KeyCode::Enter)), None);
        assert_eq!(picker.handle_key(&key(KeyCode::Right)), None);
    }

    #[test]
    fn mouse_click_above_offset_is_ignored() {
        let mut picker = sample();
        picker.set_y_offset(4);

        let msg = picker.handle_mouse(&MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 3,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(msg, None);
        assert_eq!(picker.selected_index(), 1);
    }

    #[test]
    fn mouse_wheel_updates_selected_tree_item() {
        let mut picker = sample();

        assert_eq!(
            picker.handle_mouse(&MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 0,
                row: 2,
                modifiers: KeyModifiers::NONE,
            }),
            None
        );
        assert_eq!(picker.selected_index(), 2);

        assert_eq!(
            picker.handle_mouse(&MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 0,
                row: 2,
                modifiers: KeyModifiers::NONE,
            }),
            None
        );
        assert_eq!(picker.selected_index(), 1);
    }

    #[test]
    fn mouse_wheel_above_offset_is_ignored() {
        let mut picker = sample();
        picker.set_y_offset(4);

        let msg = picker.handle_mouse(&MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 3,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(msg, None);
        assert_eq!(picker.selected_index(), 1);
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

    #[test]
    fn element_respects_max_items_window() {
        let items = (0..5)
            .map(|idx| TreePickerItem::leaf(format!("file-{idx}.rs")))
            .collect::<Vec<_>>();
        let el: Element<()> = TreePicker::without_title()
            .items(items)
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

        assert!(text.contains("file-1.rs"));
        assert!(text.contains("file-2.rs"));
        assert!(text.contains("file-3.rs"));
        assert!(!text.contains("file-0.rs"));
        assert!(!text.contains("file-4.rs"));
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
            .map(|idx| TreePickerItem::leaf(format!("file-{idx}.rs")))
            .collect::<Vec<_>>();
        let el: Element<()> = TreePicker::without_title()
            .items(items)
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
        assert!(text.contains("file-2.rs"), "{text:?}");
        assert!(text.contains("file-4.rs"), "{text:?}");
        assert!(text.contains("5/6"), "{text:?}");
        assert!(!text.contains("file-0.rs"), "{text:?}");
    }

    #[test]
    fn element_with_height_fill_height_pads_empty_rows() {
        let el: Element<()> = TreePicker::without_title()
            .item(TreePickerItem::leaf("only.rs"))
            .fill_height(true)
            .element_with_height(3);

        let Element::Box(column) = el else {
            panic!("expected column");
        };
        let rows = column
            .children
            .iter()
            .filter_map(Element::text_content)
            .collect::<Vec<_>>();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], "    only.rs");
        assert_eq!(rows[1], "");
        assert_eq!(rows[2], "");
    }
}
