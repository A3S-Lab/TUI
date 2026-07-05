use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::{KeyEvent, MouseEvent, MouseEventKind};
use crate::style::{fit_visible, Color, Style};
use crossterm::event::KeyCode;

pub struct Select {
    items: Vec<String>,
    cursor: usize,
    focused: bool,
    y_offset: u16,
    number_shortcuts: bool,
}

#[derive(Debug, Clone)]
pub enum SelectMsg {
    Selected(usize, String),
}

impl Select {
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(|i| i.into()).collect(),
            cursor: 0,
            focused: true,
            y_offset: 0,
            number_shortcuts: false,
        }
    }

    pub fn with_selected(mut self, selected: usize) -> Self {
        self.cursor = selected.min(self.max_cursor());
        self
    }

    pub fn with_number_shortcuts(mut self) -> Self {
        self.number_shortcuts = true;
        self
    }

    pub fn focus(&mut self) {
        self.focused = true;
    }
    pub fn blur(&mut self) {
        self.focused = false;
    }
    pub fn selected_index(&self) -> usize {
        self.normalized_cursor()
    }
    pub fn selected_value(&self) -> &str {
        self.selected_value_opt().unwrap_or("")
    }
    pub fn selected_value_opt(&self) -> Option<&str> {
        self.items.get(self.normalized_cursor()).map(String::as_str)
    }
    pub fn cursor(&self) -> usize {
        self.normalized_cursor()
    }

    /// Set the vertical offset for mouse click calculations.
    pub fn set_y_offset(&mut self, y: u16) {
        self.y_offset = y;
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<SelectMsg> {
        if !self.focused {
            return None;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = self.cursor.saturating_sub(1).min(self.max_cursor());
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.cursor = self
                    .normalized_cursor()
                    .saturating_add(1)
                    .min(self.max_cursor());
                None
            }
            KeyCode::Enter => self.selected_msg(),
            KeyCode::Char(c) if self.number_shortcuts => {
                let idx = number_shortcut_index(c)?;
                if idx < self.items.len() {
                    self.cursor = idx;
                    self.selected_msg()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Handle mouse events (click to select).
    pub fn handle_mouse(&mut self, mouse: &MouseEvent) -> Option<SelectMsg> {
        if !self.focused {
            return None;
        }
        match mouse.kind {
            MouseEventKind::Down(crate::event::MouseButton::Left) => {
                let row = super::relative_mouse_row(mouse.row, self.y_offset)?;
                if row < self.items.len() {
                    self.cursor = row;
                    Some(SelectMsg::Selected(
                        self.cursor,
                        self.items[self.cursor].clone(),
                    ))
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

        let cursor = self.normalized_cursor();
        let range = self.visible_range(height);

        self.items
            .iter()
            .enumerate()
            .skip(range.start)
            .take(range.len())
            .map(|(idx, item)| {
                let prefix = if idx == cursor { ">" } else { " " };
                let raw = if self.number_shortcuts {
                    match number_shortcut_label(idx) {
                        Some(label) => fit_visible(&format!("{prefix} {label} {item}"), width),
                        None => fit_visible(&format!("{prefix}   {item}"), width),
                    }
                } else {
                    fit_visible(&format!("{prefix} {item}"), width)
                };
                if idx == cursor && self.focused {
                    Style::new().fg(Color::Cyan).bold().render(&raw)
                } else {
                    raw
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        self.element_with_height(self.items.len())
    }

    pub fn element_with_height<Msg>(&self, height: usize) -> Element<Msg> {
        let cursor = self.normalized_cursor();
        let range = self.visible_range(height);
        let children: Vec<Element<Msg>> = self
            .items
            .iter()
            .enumerate()
            .skip(range.start)
            .take(range.len())
            .map(|(i, item)| {
                let prefix = if i == cursor { "▸ " } else { "  " };
                let text = if self.number_shortcuts {
                    match number_shortcut_label(i) {
                        Some(label) => format!("{prefix}{label} {item}"),
                        None => format!("{prefix}  {item}"),
                    }
                } else {
                    format!("{}{}", prefix, item)
                };
                if i == cursor && self.focused {
                    Element::Text(TextElement::new(text).bold().fg(Color::Cyan))
                } else {
                    Element::Text(TextElement::new(text))
                }
            })
            .collect();

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(children),
        )
    }

    fn selected_msg(&self) -> Option<SelectMsg> {
        let cursor = self.normalized_cursor();
        self.items
            .get(cursor)
            .cloned()
            .map(|item| SelectMsg::Selected(cursor, item))
    }

    fn max_cursor(&self) -> usize {
        self.items.len().saturating_sub(1)
    }

    fn normalized_cursor(&self) -> usize {
        self.cursor.min(self.max_cursor())
    }

    fn visible_range(&self, height: usize) -> std::ops::Range<usize> {
        if height == 0 || self.items.is_empty() {
            return 0..0;
        }

        let cursor = self.normalized_cursor();
        let visible = height.min(self.items.len());
        let start = if self.items.len() <= visible {
            0
        } else {
            cursor
                .saturating_sub(visible - 1)
                .min(self.items.len() - visible)
        };
        start..start.saturating_add(visible).min(self.items.len())
    }
}

fn number_shortcut_index(c: char) -> Option<usize> {
    match c {
        '1'..='9' => Some((c as u8 - b'1') as usize),
        '0' => Some(9),
        _ => None,
    }
}

fn number_shortcut_label(idx: usize) -> Option<char> {
    match idx {
        0..=8 => Some((b'1' + idx as u8) as char),
        9 => Some('0'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::visible_len;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn initial_state() {
        let select = Select::new(vec!["a", "b", "c"]);
        assert_eq!(select.selected_index(), 0);
        assert_eq!(select.selected_value(), "a");
    }

    #[test]
    fn initializes_selected_state() {
        let select = Select::new(vec!["a", "b", "c"]).with_selected(9);

        assert_eq!(select.selected_index(), 2);
        assert_eq!(select.cursor(), 2);
    }

    #[test]
    fn navigate_down() {
        let mut select = Select::new(vec!["a", "b", "c"]);
        select.handle_key(&key(KeyCode::Down));
        assert_eq!(select.selected_index(), 1);
        select.handle_key(&key(KeyCode::Char('j')));
        assert_eq!(select.selected_index(), 2);
    }

    #[test]
    fn navigate_up() {
        let mut select = Select::new(vec!["a", "b", "c"]);
        select.handle_key(&key(KeyCode::Down));
        select.handle_key(&key(KeyCode::Down));
        select.handle_key(&key(KeyCode::Up));
        assert_eq!(select.selected_index(), 1);
    }

    #[test]
    fn bounds_check() {
        let mut select = Select::new(vec!["a", "b"]);
        select.handle_key(&key(KeyCode::Up));
        assert_eq!(select.selected_index(), 0);
        select.handle_key(&key(KeyCode::Down));
        select.handle_key(&key(KeyCode::Down));
        assert_eq!(select.selected_index(), 1);
    }

    #[test]
    fn stale_cursor_navigation_clamps_to_items() {
        let mut select = Select::new(vec!["a", "b"]);
        select.cursor = usize::MAX;

        select.handle_key(&key(KeyCode::Down));
        assert_eq!(select.selected_index(), 1);

        select.cursor = usize::MAX;
        select.handle_key(&key(KeyCode::Up));
        assert_eq!(select.selected_index(), 1);
    }

    #[test]
    fn stale_cursor_is_normalized_for_selection_rendering_and_enter() {
        let mut select = Select::new(vec!["a", "b"]).with_number_shortcuts();
        select.cursor = usize::MAX;

        assert_eq!(select.selected_index(), 1);
        assert_eq!(select.cursor(), 1);
        assert_eq!(select.selected_value(), "b");
        assert_eq!(select.selected_value_opt(), Some("b"));
        assert!(matches!(
            select.handle_key(&key(KeyCode::Enter)),
            Some(SelectMsg::Selected(1, value)) if value == "b"
        ));

        let plain = crate::style::strip_ansi(&select.view(20, 5));
        assert!(plain.contains("> 2 b"));
        assert!(!plain.contains("> 1 a"));

        let Element::Box(box_el) = select.element::<()>() else {
            panic!("expected box element");
        };
        let Element::Text(last_item) = box_el.children.last().expect("expected last item") else {
            panic!("expected select item");
        };
        assert_eq!(last_item.content, "▸ 2 b");

        select.handle_key(&key(KeyCode::Up));
        assert_eq!(select.selected_index(), 1);
    }

    #[test]
    fn empty_select_has_no_selected_value_and_ignores_selection_input() {
        let mut select = Select::new(Vec::<&str>::new()).with_selected(9);

        assert_eq!(select.selected_index(), 0);
        assert_eq!(select.selected_value(), "");
        assert_eq!(select.selected_value_opt(), None);
        assert!(select.handle_key(&key(KeyCode::Enter)).is_none());
        assert!(select
            .handle_mouse(&MouseEvent {
                kind: MouseEventKind::Down(crate::event::MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            })
            .is_none());
        assert_eq!(select.view(10, 2), "");

        let Element::Box(box_el) = select.element::<()>() else {
            panic!("expected box element");
        };
        assert!(box_el.children.is_empty());
    }

    #[test]
    fn enter_selects() {
        let mut select = Select::new(vec!["x", "y"]);
        select.handle_key(&key(KeyCode::Down));
        let msg = select.handle_key(&key(KeyCode::Enter));
        assert!(matches!(msg, Some(SelectMsg::Selected(1, _))));
    }

    #[test]
    fn mouse_click_above_offset_is_ignored() {
        let mut select = Select::new(vec!["x", "y"]).with_selected(1);
        select.set_y_offset(4);

        let msg = select.handle_mouse(&MouseEvent {
            kind: MouseEventKind::Down(crate::event::MouseButton::Left),
            column: 0,
            row: 3,
            modifiers: KeyModifiers::NONE,
        });

        assert!(msg.is_none());
        assert_eq!(select.selected_index(), 1);
    }

    #[test]
    fn number_shortcuts_are_opt_in() {
        let mut select = Select::new(vec!["one", "two", "three"]);

        let msg = select.handle_key(&key(KeyCode::Char('2')));

        assert!(msg.is_none());
        assert_eq!(select.selected_index(), 0);
    }

    #[test]
    fn number_shortcuts_select_by_visible_index() {
        let mut select = Select::new(vec![
            "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        ])
        .with_number_shortcuts();

        let msg = select.handle_key(&key(KeyCode::Char('3')));
        assert!(matches!(msg, Some(SelectMsg::Selected(2, _))));
        assert_eq!(select.selected_index(), 2);

        let msg = select.handle_key(&key(KeyCode::Char('0')));
        assert!(matches!(msg, Some(SelectMsg::Selected(9, _))));
        assert_eq!(select.selected_value(), "ten");
    }

    #[test]
    fn blur_ignores_input() {
        let mut select = Select::new(vec!["a", "b"]);
        select.blur();
        select.handle_key(&key(KeyCode::Down));
        assert_eq!(select.selected_index(), 0);
    }

    #[test]
    fn view_renders_cursor_and_truncates() {
        let select = Select::new(vec!["a very long option", "b"]).with_selected(1);
        let plain = crate::style::strip_ansi(&select.view(10, 5));

        assert!(plain.contains("> b"));
        for line in plain.lines() {
            assert!(visible_len(line) <= 10, "{line:?}");
        }
    }

    #[test]
    fn view_renders_number_shortcuts_when_enabled() {
        let select = Select::new(vec!["cpu", "memory", "network"]).with_number_shortcuts();
        let plain = crate::style::strip_ansi(&select.view(20, 5));

        assert!(plain.contains("> 1 cpu"));
        assert!(plain.contains("  2 memory"));
        assert!(plain.contains("  3 network"));
    }

    #[test]
    fn view_scrolls_to_cursor() {
        let select = Select::new(vec!["one", "two", "three", "four"]).with_selected(3);
        let plain = crate::style::strip_ansi(&select.view(16, 2));

        assert!(!plain.contains("one"));
        assert!(plain.contains("> four"));
    }

    #[test]
    fn element_with_height_scrolls_to_cursor() {
        let select = Select::new(vec!["one", "two", "three", "four"])
            .with_selected(3)
            .with_number_shortcuts();

        let Element::Box(column) = select.element_with_height::<()>(2) else {
            panic!("expected box element");
        };
        let text = column
            .children
            .iter()
            .filter_map(Element::text_content)
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(column.children.len(), 2);
        assert!(text.contains("  3 three"));
        assert!(text.contains("▸ 4 four"));
        assert!(!text.contains("one"));
        assert!(!text.contains("two"));
    }
}
