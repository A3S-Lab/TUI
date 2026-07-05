use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::KeyEvent;
use crate::style::{fit_visible, Color, Style};
use crossterm::event::KeyCode;

pub struct MultiSelect {
    items: Vec<String>,
    cursor: usize,
    checked: Vec<bool>,
    focused: bool,
    number_shortcuts: bool,
}

#[derive(Debug, Clone)]
pub enum MultiSelectMsg {
    Toggle(usize),
    Submit(Vec<usize>),
}

impl MultiSelect {
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        let items: Vec<String> = items.into_iter().map(|i| i.into()).collect();
        let checked = vec![false; items.len()];
        Self {
            items,
            cursor: 0,
            checked,
            focused: true,
            number_shortcuts: false,
        }
    }

    pub fn focus(&mut self) {
        self.focused = true;
    }
    pub fn blur(&mut self) {
        self.focused = false;
    }

    pub fn selected_indices(&self) -> Vec<usize> {
        self.checked
            .iter()
            .take(self.items.len())
            .enumerate()
            .filter_map(|(i, &b)| if b { Some(i) } else { None })
            .collect()
    }

    pub fn with_checked(mut self, checked: Vec<bool>) -> Self {
        self.checked = self
            .items
            .iter()
            .enumerate()
            .map(|(idx, _)| checked.get(idx).copied().unwrap_or(false))
            .collect();
        self
    }

    pub fn with_number_shortcuts(mut self) -> Self {
        self.number_shortcuts = true;
        self
    }

    pub fn set_checked(&mut self, index: usize, checked: bool) {
        if let Some(slot) = self.checked.get_mut(index) {
            *slot = checked;
        }
    }

    pub fn is_checked(&self, index: usize) -> bool {
        if index >= self.items.len() {
            return false;
        }
        self.checked.get(index).copied().unwrap_or(false)
    }

    pub fn checked(&self) -> &[bool] {
        &self.checked
    }

    pub fn cursor(&self) -> usize {
        self.normalized_cursor()
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<MultiSelectMsg> {
        if !self.focused {
            return None;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = self.cursor.saturating_sub(1).min(self.max_cursor());
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.cursor = self.cursor.saturating_add(1).min(self.max_cursor());
                None
            }
            KeyCode::Char(' ') => {
                self.cursor = self.normalized_cursor();
                self.toggle_index(self.cursor)
            }
            KeyCode::Char(c) if self.number_shortcuts => {
                let idx = number_shortcut_index(c)?;
                if idx < self.items.len() {
                    self.cursor = idx;
                    self.toggle_index(idx)
                } else {
                    None
                }
            }
            KeyCode::Enter => Some(MultiSelectMsg::Submit(self.selected_indices())),
            _ => None,
        }
    }

    fn toggle_index(&mut self, index: usize) -> Option<MultiSelectMsg> {
        self.normalize_checked_len();
        let checked = self.checked.get_mut(index)?;
        *checked = !*checked;
        Some(MultiSelectMsg::Toggle(index))
    }

    fn max_cursor(&self) -> usize {
        self.items.len().saturating_sub(1)
    }

    fn normalized_cursor(&self) -> usize {
        self.cursor.min(self.max_cursor())
    }

    fn normalize_checked_len(&mut self) {
        self.checked.resize(self.items.len(), false);
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
                let cursor_marker = if idx == cursor { ">" } else { " " };
                let check = if self.is_checked(idx) { "[x]" } else { "[ ]" };
                let raw = if self.number_shortcuts {
                    match number_shortcut_label(idx) {
                        Some(label) => {
                            fit_visible(&format!("{cursor_marker} {label} {check} {item}"), width)
                        }
                        None => fit_visible(&format!("{cursor_marker}   {check} {item}"), width),
                    }
                } else {
                    fit_visible(&format!("{cursor_marker} {check} {item}"), width)
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
                let cursor_marker = if i == cursor { "▸" } else { " " };
                let check = if self.is_checked(i) { "[x]" } else { "[ ]" };
                let text = if self.number_shortcuts {
                    match number_shortcut_label(i) {
                        Some(label) => format!("{cursor_marker} {label} {check} {item}"),
                        None => format!("{cursor_marker}   {check} {item}"),
                    }
                } else {
                    format!("{} {} {}", cursor_marker, check, item)
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
        let ms = MultiSelect::new(vec!["a", "b", "c"]);
        assert!(ms.selected_indices().is_empty());
    }

    #[test]
    fn toggle_with_space() {
        let mut ms = MultiSelect::new(vec!["a", "b", "c"]);
        ms.handle_key(&key(KeyCode::Char(' ')));
        assert_eq!(ms.selected_indices(), vec![0]);
        ms.handle_key(&key(KeyCode::Down));
        ms.handle_key(&key(KeyCode::Char(' ')));
        assert_eq!(ms.selected_indices(), vec![0, 1]);
    }

    #[test]
    fn number_shortcuts_are_opt_in() {
        let mut ms = MultiSelect::new(vec!["one", "two", "three"]);

        let msg = ms.handle_key(&key(KeyCode::Char('2')));

        assert!(msg.is_none());
        assert_eq!(ms.cursor(), 0);
        assert!(ms.selected_indices().is_empty());
    }

    #[test]
    fn number_shortcuts_toggle_by_visible_index() {
        let mut ms = MultiSelect::new(vec![
            "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        ])
        .with_number_shortcuts();

        let msg = ms.handle_key(&key(KeyCode::Char('3')));
        assert!(matches!(msg, Some(MultiSelectMsg::Toggle(2))));
        assert_eq!(ms.cursor(), 2);
        assert_eq!(ms.selected_indices(), vec![2]);

        let msg = ms.handle_key(&key(KeyCode::Char('0')));
        assert!(matches!(msg, Some(MultiSelectMsg::Toggle(9))));
        assert_eq!(ms.cursor(), 9);
        assert_eq!(ms.selected_indices(), vec![2, 9]);
    }

    #[test]
    fn untoggle() {
        let mut ms = MultiSelect::new(vec!["a", "b"]);
        ms.handle_key(&key(KeyCode::Char(' ')));
        ms.handle_key(&key(KeyCode::Char(' ')));
        assert!(ms.selected_indices().is_empty());
    }

    #[test]
    fn submit_returns_selected() {
        let mut ms = MultiSelect::new(vec!["x", "y", "z"]);
        ms.handle_key(&key(KeyCode::Char(' ')));
        ms.handle_key(&key(KeyCode::Down));
        ms.handle_key(&key(KeyCode::Down));
        ms.handle_key(&key(KeyCode::Char(' ')));
        let msg = ms.handle_key(&key(KeyCode::Enter));
        assert!(matches!(msg, Some(MultiSelectMsg::Submit(v)) if v == vec![0, 2]));
    }

    #[test]
    fn initializes_checked_state() {
        let ms = MultiSelect::new(vec!["a", "b", "c"]).with_checked(vec![true, false, true]);

        assert_eq!(ms.selected_indices(), vec![0, 2]);
        assert!(ms.is_checked(0));
        assert!(!ms.is_checked(9));
    }

    #[test]
    fn view_renders_checks_and_cursor() {
        let ms = MultiSelect::new(vec!["alpha", "beta"]).with_checked(vec![true, false]);
        let plain = crate::style::strip_ansi(&ms.view(20, 5));

        assert!(plain.contains("> [x] alpha"));
        assert!(plain.contains("  [ ] beta"));
    }

    #[test]
    fn view_renders_number_shortcuts_when_enabled() {
        let ms = MultiSelect::new(vec!["status", "name", "cid"])
            .with_checked(vec![true, false, true])
            .with_number_shortcuts();
        let plain = crate::style::strip_ansi(&ms.view(24, 5));

        assert!(plain.contains("> 1 [x] status"));
        assert!(plain.contains("  2 [ ] name"));
        assert!(plain.contains("  3 [x] cid"));
    }

    #[test]
    fn view_truncates_to_width() {
        let ms = MultiSelect::new(vec!["a very long option name"]);

        for line in crate::style::strip_ansi(&ms.view(10, 5)).lines() {
            assert!(visible_len(line) <= 10, "{line:?}");
        }
    }

    #[test]
    fn view_scrolls_to_cursor() {
        let mut ms = MultiSelect::new(vec!["one", "two", "three", "four"]);
        ms.handle_key(&KeyEvent {
            code: KeyCode::Down,
            modifiers: crossterm::event::KeyModifiers::empty(),
        });
        ms.handle_key(&KeyEvent {
            code: KeyCode::Down,
            modifiers: crossterm::event::KeyModifiers::empty(),
        });
        ms.handle_key(&KeyEvent {
            code: KeyCode::Down,
            modifiers: crossterm::event::KeyModifiers::empty(),
        });

        let plain = crate::style::strip_ansi(&ms.view(16, 2));

        assert!(!plain.contains("one"));
        assert!(plain.contains("> [ ] four"));
    }

    #[test]
    fn element_with_height_scrolls_to_cursor() {
        let mut ms = MultiSelect::new(vec!["one", "two", "three", "four"])
            .with_checked(vec![false, true, false, true])
            .with_number_shortcuts();
        ms.handle_key(&key(KeyCode::Down));
        ms.handle_key(&key(KeyCode::Down));
        ms.handle_key(&key(KeyCode::Down));

        let Element::Box(column) = ms.element_with_height::<()>(2) else {
            panic!("expected box element");
        };
        let text = column
            .children
            .iter()
            .filter_map(Element::text_content)
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(column.children.len(), 2);
        assert!(text.contains("  3 [ ] three"));
        assert!(text.contains("▸ 4 [x] four"));
        assert!(!text.contains("one"));
        assert!(!text.contains("two"));
    }

    #[test]
    fn navigation_bounds() {
        let mut ms = MultiSelect::new(vec!["a", "b"]);
        ms.handle_key(&key(KeyCode::Up));
        assert_eq!(ms.cursor, 0);
        ms.handle_key(&key(KeyCode::Down));
        ms.handle_key(&key(KeyCode::Down));
        assert_eq!(ms.cursor, 1);
    }

    #[test]
    fn stale_cursor_navigation_clamps_to_items() {
        let mut ms = MultiSelect::new(vec!["a", "b"]);
        ms.cursor = usize::MAX;

        ms.handle_key(&key(KeyCode::Down));
        assert_eq!(ms.cursor(), 1);

        ms.cursor = usize::MAX;
        ms.handle_key(&key(KeyCode::Up));
        assert_eq!(ms.cursor(), 1);
    }

    #[test]
    fn stale_cursor_is_normalized_for_rendering_and_toggle() {
        let mut ms = MultiSelect::new(vec!["a", "b"]).with_number_shortcuts();
        ms.cursor = usize::MAX;

        assert_eq!(ms.cursor(), 1);

        let plain = crate::style::strip_ansi(&ms.view(20, 5));
        assert!(plain.contains("> 2 [ ] b"));
        assert!(!plain.contains("> 1 [ ] a"));

        let Element::Box(box_el) = ms.element::<()>() else {
            panic!("expected box element");
        };
        let Element::Text(last_item) = box_el.children.last().expect("expected last item") else {
            panic!("expected multi-select item");
        };
        assert_eq!(last_item.content, "▸ 2 [ ] b");

        assert!(matches!(
            ms.handle_key(&key(KeyCode::Char(' '))),
            Some(MultiSelectMsg::Toggle(1))
        ));
        assert_eq!(ms.cursor(), 1);
        assert_eq!(ms.selected_indices(), vec![1]);
    }

    #[test]
    fn stale_extra_checked_state_is_ignored() {
        let mut ms = MultiSelect::new(vec!["a", "b"]);
        ms.checked = vec![true, false, true];

        assert_eq!(ms.selected_indices(), vec![0]);
        assert!(!ms.is_checked(2));
        assert!(matches!(
            ms.handle_key(&key(KeyCode::Enter)),
            Some(MultiSelectMsg::Submit(selected)) if selected == vec![0]
        ));
    }

    #[test]
    fn stale_short_checked_state_is_extended_for_toggle() {
        let mut ms = MultiSelect::new(vec!["a", "b"]).with_number_shortcuts();
        ms.checked = vec![true];
        ms.cursor = 1;

        let plain = crate::style::strip_ansi(&ms.view(20, 5));
        assert!(plain.contains("> 2 [ ] b"));

        let Element::Box(box_el) = ms.element::<()>() else {
            panic!("expected box element");
        };
        let Element::Text(last_item) = box_el.children.last().expect("expected last item") else {
            panic!("expected multi-select item");
        };
        assert_eq!(last_item.content, "▸ 2 [ ] b");

        assert!(matches!(
            ms.handle_key(&key(KeyCode::Char('2'))),
            Some(MultiSelectMsg::Toggle(1))
        ));
        assert_eq!(ms.checked(), &[true, true]);
        assert_eq!(ms.selected_indices(), vec![0, 1]);
    }

    #[test]
    fn empty_multi_select_ignores_toggle_input() {
        let mut ms = MultiSelect::new(Vec::<&str>::new()).with_number_shortcuts();

        assert_eq!(ms.cursor(), 0);
        assert!(ms.selected_indices().is_empty());
        assert!(ms.handle_key(&key(KeyCode::Char(' '))).is_none());
        assert!(ms.handle_key(&key(KeyCode::Char('1'))).is_none());
        assert!(matches!(
            ms.handle_key(&key(KeyCode::Enter)),
            Some(MultiSelectMsg::Submit(selected)) if selected.is_empty()
        ));
        assert_eq!(ms.view(10, 3), "");

        let Element::Box(box_el) = ms.element::<()>() else {
            panic!("expected box element");
        };
        assert!(box_el.children.is_empty());
    }

    #[test]
    fn blur_ignores_input() {
        let mut ms = MultiSelect::new(vec!["a"]);
        ms.blur();
        ms.handle_key(&key(KeyCode::Char(' ')));
        assert!(ms.selected_indices().is_empty());
    }
}
