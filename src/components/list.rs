use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::Color;

pub struct List<T> {
    items: Vec<T>,
    cursor: usize,
    height: usize,
    offset: usize,
}

#[derive(Debug, Clone)]
pub enum ListMsg {
    Up,
    Down,
    Select(usize),
    PageUp,
    PageDown,
    Home,
    End,
}

impl<T: std::fmt::Display> List<T> {
    pub fn new(items: Vec<T>, height: usize) -> Self {
        Self {
            items,
            cursor: 0,
            height,
            offset: 0,
        }
    }

    pub fn selected(&self) -> Option<&T> {
        self.items.get(self.cursor)
    }

    pub fn selected_index(&self) -> usize {
        self.cursor
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.cursor = self.cursor.min(self.items.len().saturating_sub(1));
        self.adjust_offset();
    }

    pub fn update(&mut self, msg: ListMsg) {
        match msg {
            ListMsg::Up => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            ListMsg::Down => {
                if self.cursor + 1 < self.items.len() {
                    self.cursor += 1;
                }
            }
            ListMsg::Select(i) => {
                if i < self.items.len() {
                    self.cursor = i;
                }
            }
            ListMsg::PageUp => {
                self.cursor = self.cursor.saturating_sub(self.height);
            }
            ListMsg::PageDown => {
                self.cursor =
                    (self.cursor + self.height).min(self.items.len().saturating_sub(1));
            }
            ListMsg::Home => {
                self.cursor = 0;
            }
            ListMsg::End => {
                self.cursor = self.items.len().saturating_sub(1);
            }
        }
        self.adjust_offset();
    }

    pub fn view<F>(&self, render_item: F) -> String
    where
        F: Fn(&T, bool) -> String,
    {
        let end = (self.offset + self.height).min(self.items.len());
        let mut lines = Vec::new();
        for i in self.offset..end {
            let selected = i == self.cursor;
            lines.push(render_item(&self.items[i], selected));
        }
        lines.join("\n")
    }

    /// Render as an Element tree. Each item is displayed with a cursor indicator.
    pub fn element<Msg>(&self) -> Element<Msg> {
        let end = (self.offset + self.height).min(self.items.len());
        let children: Vec<Element<Msg>> = (self.offset..end)
            .map(|i| {
                let selected = i == self.cursor;
                let prefix = if selected { "▸ " } else { "  " };
                let text = format!("{}{}", prefix, self.items[i]);
                if selected {
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

    fn adjust_offset(&mut self) {
        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if self.cursor >= self.offset + self.height {
            self.offset = self.cursor - self.height + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let list = List::new(vec!["a", "b", "c"], 5);
        assert_eq!(list.selected_index(), 0);
        assert_eq!(list.selected(), Some(&"a"));
    }

    #[test]
    fn navigate_down() {
        let mut list = List::new(vec!["a", "b", "c"], 5);
        list.update(ListMsg::Down);
        assert_eq!(list.selected_index(), 1);
    }

    #[test]
    fn navigate_up() {
        let mut list = List::new(vec!["a", "b", "c"], 5);
        list.update(ListMsg::Down);
        list.update(ListMsg::Up);
        assert_eq!(list.selected_index(), 0);
    }

    #[test]
    fn bounds_check() {
        let mut list = List::new(vec!["a", "b"], 5);
        list.update(ListMsg::Up);
        assert_eq!(list.selected_index(), 0);
        list.update(ListMsg::Down);
        list.update(ListMsg::Down);
        assert_eq!(list.selected_index(), 1);
    }

    #[test]
    fn select_specific() {
        let mut list = List::new(vec!["a", "b", "c"], 5);
        list.update(ListMsg::Select(2));
        assert_eq!(list.selected_index(), 2);
    }

    #[test]
    fn home_and_end() {
        let mut list = List::new(vec!["a", "b", "c", "d"], 5);
        list.update(ListMsg::End);
        assert_eq!(list.selected_index(), 3);
        list.update(ListMsg::Home);
        assert_eq!(list.selected_index(), 0);
    }

    #[test]
    fn scrolling_adjusts_offset() {
        let mut list = List::new(vec!["a", "b", "c", "d", "e"], 3);
        list.update(ListMsg::Down);
        list.update(ListMsg::Down);
        list.update(ListMsg::Down);
        assert_eq!(list.selected_index(), 3);
        assert!(list.offset > 0);
    }

    #[test]
    fn set_items() {
        let mut list = List::new(vec!["a", "b", "c"], 5);
        list.update(ListMsg::End);
        list.set_items(vec!["x", "y"]);
        assert_eq!(list.selected_index(), 1);
        assert_eq!(list.items().len(), 2);
    }
}
