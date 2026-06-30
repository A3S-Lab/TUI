use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::style::{fit_visible, truncate_visible, visible_len, Color, Style};
use crossterm::event::KeyCode;

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
        self.depth = depth;
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
        self.max_items = Some(max_items.max(1));
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

    pub fn depth_indent(mut self, indent: usize) -> Self {
        self.depth_indent = indent.max(1);
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

    pub fn set_y_offset(&mut self, y: u16) {
        self.y_offset = y;
    }

    pub fn items_value(&self) -> &[TreePickerItem] {
        &self.items
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected_item(&self) -> Option<&TreePickerItem> {
        self.items.get(self.selected)
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<TreePickerMsg> {
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
            KeyCode::Right => self.branch_msg(TreePickerMsg::Opened, false),
            KeyCode::Left => self.branch_msg(TreePickerMsg::Closed, true),
            KeyCode::Enter => self.activate_selected(),
            KeyCode::Esc => Some(TreePickerMsg::Cancelled),
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<TreePickerMsg> {
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

        for (index, item) in self.items.iter().enumerate() {
            let mut text = TextElement::new(self.plain_item_line(index, None));
            if index == self.selected {
                text = text.fg(self.selected_fg).bg(self.selected_bg).bold();
            } else if item.disabled {
                text = text.fg(self.disabled_color);
            } else {
                text = text.fg(item.color.unwrap_or(self.default_item_color(item)));
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
            Style::new()
                .fg(item.color.unwrap_or(self.default_item_color(item)))
                .render(&raw)
        }
    }

    fn plain_item_line(&self, index: usize, width: Option<usize>) -> String {
        let Some(item) = self.items.get(index) else {
            return String::new();
        };
        let marker = match item.kind {
            TreePickerItemKind::Branch { open: true } => self.open_marker.as_str(),
            TreePickerItemKind::Branch { open: false } => self.closed_marker.as_str(),
            TreePickerItemKind::Leaf => self.leaf_marker.as_str(),
        };
        let prefix = format!(
            "{}{}{} ",
            " ".repeat(self.indent),
            " ".repeat(item.depth.saturating_mul(self.depth_indent)),
            marker
        );
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

    fn activate_selected(&self) -> Option<TreePickerMsg> {
        let item = self.items.get(self.selected)?;
        if item.disabled {
            return None;
        }
        if item.is_branch() {
            Some(TreePickerMsg::Toggled(self.selected))
        } else {
            Some(TreePickerMsg::Selected(self.selected))
        }
    }

    fn branch_msg(
        &self,
        f: impl FnOnce(usize) -> TreePickerMsg,
        expected_open: bool,
    ) -> Option<TreePickerMsg> {
        let item = self.items.get(self.selected)?;
        if item.disabled || !item.is_branch() || item.is_open() != expected_open {
            None
        } else {
            Some(f(self.selected))
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

    fn default_item_color(&self, item: &TreePickerItem) -> Color {
        if item.is_branch() {
            self.branch_color
        } else {
            self.leaf_color
        }
    }
}

impl Default for TreePicker {
    fn default() -> Self {
        Self::without_title()
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
    fn disabled_items_do_not_emit_actions() {
        let mut picker = TreePicker::new("@ file")
            .item(TreePickerItem::branch("target").disabled(true))
            .item(TreePickerItem::leaf("main.rs"))
            .selected(0);

        assert_eq!(picker.handle_key(&key(KeyCode::Enter)), None);
        assert_eq!(picker.handle_key(&key(KeyCode::Right)), None);
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
