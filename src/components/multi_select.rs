use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::event::KeyEvent;
use crate::style::{truncate_visible, visible_len, Color, Style};
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
        self.checked.get(index).copied().unwrap_or(false)
    }

    pub fn checked(&self) -> &[bool] {
        &self.checked
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<MultiSelectMsg> {
        if !self.focused {
            return None;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = self.cursor.saturating_sub(1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor + 1 < self.items.len() {
                    self.cursor += 1;
                }
                None
            }
            KeyCode::Char(' ') => {
                self.checked[self.cursor] = !self.checked[self.cursor];
                Some(MultiSelectMsg::Toggle(self.cursor))
            }
            KeyCode::Char(c) if self.number_shortcuts => {
                let idx = number_shortcut_index(c)?;
                if idx < self.items.len() {
                    self.cursor = idx;
                    self.checked[idx] = !self.checked[idx];
                    Some(MultiSelectMsg::Toggle(idx))
                } else {
                    None
                }
            }
            KeyCode::Enter => Some(MultiSelectMsg::Submit(self.selected_indices())),
            _ => None,
        }
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        let start = if self.items.len() <= height {
            0
        } else {
            self.cursor
                .saturating_sub(height - 1)
                .min(self.items.len() - height)
        };

        self.items
            .iter()
            .enumerate()
            .skip(start)
            .take(height)
            .map(|(idx, item)| {
                let cursor = if idx == self.cursor { ">" } else { " " };
                let check = if self.checked[idx] { "[x]" } else { "[ ]" };
                let raw = if self.number_shortcuts {
                    match number_shortcut_label(idx) {
                        Some(label) => {
                            pad_or_truncate(&format!("{cursor} {label} {check} {item}"), width)
                        }
                        None => pad_or_truncate(&format!("{cursor}   {check} {item}"), width),
                    }
                } else {
                    pad_or_truncate(&format!("{cursor} {check} {item}"), width)
                };
                if idx == self.cursor && self.focused {
                    Style::new().fg(Color::Cyan).bold().render(&raw)
                } else {
                    raw
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let children: Vec<Element<Msg>> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let cursor_marker = if i == self.cursor { "▸" } else { " " };
                let check = if self.checked[i] { "[x]" } else { "[ ]" };
                let text = if self.number_shortcuts {
                    match number_shortcut_label(i) {
                        Some(label) => format!("{cursor_marker} {label} {check} {item}"),
                        None => format!("{cursor_marker}   {check} {item}"),
                    }
                } else {
                    format!("{} {} {}", cursor_marker, check, item)
                };
                if i == self.cursor && self.focused {
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

fn pad_or_truncate(value: &str, width: usize) -> String {
    let truncated = truncate_to_width(value, width);
    let len = visible_len(&truncated);
    if len >= width {
        truncated
    } else {
        format!("{truncated}{}", " ".repeat(width - len))
    }
}

fn truncate_to_width(value: &str, width: usize) -> String {
    truncate_visible(value, width)
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn navigation_bounds() {
        let mut ms = MultiSelect::new(vec!["a", "b"]);
        ms.handle_key(&key(KeyCode::Up));
        assert_eq!(ms.cursor, 0);
        ms.handle_key(&key(KeyCode::Down));
        ms.handle_key(&key(KeyCode::Down));
        assert_eq!(ms.cursor, 1);
    }

    #[test]
    fn blur_ignores_input() {
        let mut ms = MultiSelect::new(vec!["a"]);
        ms.blur();
        ms.handle_key(&key(KeyCode::Char(' ')));
        assert!(ms.selected_indices().is_empty());
    }
}
