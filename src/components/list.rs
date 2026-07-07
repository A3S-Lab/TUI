use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::interaction::Selectable;
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
        self.items.get(self.normalized_cursor())
    }

    pub fn selected_index(&self) -> usize {
        self.normalized_cursor()
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.adjust_offset();
    }

    pub fn update(&mut self, msg: ListMsg) {
        self.cursor = self.normalized_cursor();
        match msg {
            ListMsg::Up => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            ListMsg::Down => {
                if self.cursor.saturating_add(1) < self.items.len() {
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
                self.cursor = self
                    .cursor
                    .saturating_add(self.height)
                    .min(self.items.len().saturating_sub(1));
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
        let (start, end) = self.visible_range();
        let cursor = self.normalized_cursor();
        let mut lines = Vec::new();
        for i in start..end {
            let selected = i == cursor;
            lines.push(render_item(&self.items[i], selected));
        }
        lines.join("\n")
    }

    /// Render as an Element tree. Each item is displayed with a cursor indicator.
    pub fn element<Msg>(&self) -> Element<Msg> {
        let (start, end) = self.visible_range();
        let cursor = self.normalized_cursor();
        let children: Vec<Element<Msg>> = (start..end)
            .map(|i| {
                let selected = i == cursor;
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
        self.cursor = self.normalized_cursor();
        if self.items.is_empty() || self.height == 0 {
            self.offset = 0;
            return;
        }

        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if self.cursor >= self.offset.saturating_add(self.height) {
            self.offset = self.cursor.saturating_sub(self.height).saturating_add(1);
        }
    }

    fn visible_range(&self) -> (usize, usize) {
        if self.height == 0 || self.items.is_empty() {
            return (0, 0);
        }

        let max_start = self
            .items
            .len()
            .saturating_sub(self.height.min(self.items.len()));
        let mut start = self.offset.min(max_start);
        let cursor = self.normalized_cursor();
        if cursor < start {
            start = cursor;
        } else if cursor >= start.saturating_add(self.height) {
            start = cursor.saturating_add(1).saturating_sub(self.height);
        }
        start = start.min(max_start);
        let end = start.saturating_add(self.height).min(self.items.len());
        (start, end)
    }

    fn normalized_cursor(&self) -> usize {
        self.cursor.min(self.items.len().saturating_sub(1))
    }
}

impl<T: std::fmt::Display> Selectable for List<T> {
    fn item_count(&self) -> usize {
        self.items.len()
    }

    fn selected_index(&self) -> Option<usize> {
        (!self.items.is_empty()).then(|| self.normalized_cursor())
    }

    fn select_index(&mut self, index: usize) {
        self.cursor = index.min(self.items.len().saturating_sub(1));
        self.adjust_offset();
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
    fn zero_height_keeps_offset_at_origin() {
        let mut list = List::new(vec!["a", "b", "c"], 0);

        list.update(ListMsg::Down);
        list.update(ListMsg::PageDown);
        list.update(ListMsg::End);

        assert_eq!(list.selected_index(), 2);
        assert_eq!(list.offset, 0);
        assert_eq!(list.view(|item, selected| format!("{selected}:{item}")), "");

        let Element::Box(box_el) = list.element::<()>() else {
            panic!("expected box element");
        };
        assert!(box_el.children.is_empty());
    }

    #[test]
    fn huge_page_down_saturates_at_last_item() {
        let mut list = List::new(vec!["a", "b"], usize::MAX);
        list.update(ListMsg::Down);

        list.update(ListMsg::PageDown);

        assert_eq!(list.selected_index(), 1);
        assert_eq!(list.selected(), Some(&"b"));
    }

    #[test]
    fn stale_cursor_is_normalized_for_selection_rendering_and_navigation() {
        let mut list = List::new(vec!["a", "b", "c"], 3);
        list.cursor = usize::MAX;

        assert_eq!(list.selected_index(), 2);
        assert_eq!(list.selected(), Some(&"c"));
        assert_eq!(
            list.view(|item, selected| format!("{selected}:{item}")),
            "false:a\nfalse:b\ntrue:c"
        );

        let Element::Box(box_el) = list.element::<()>() else {
            panic!("expected box element");
        };
        let Element::Text(last_item) = box_el.children.last().expect("expected last item") else {
            panic!("expected list item");
        };
        assert_eq!(last_item.content, "▸ c");

        list.update(ListMsg::Down);
        assert_eq!(list.selected_index(), 2);

        list.update(ListMsg::Up);
        assert_eq!(list.selected_index(), 1);
    }

    #[test]
    fn stale_offset_is_normalized_for_rendering() {
        let mut list = List::new(vec!["a", "b", "c", "d"], 2);
        list.update(ListMsg::End);
        list.offset = usize::MAX;

        assert_eq!(
            list.view(|item, selected| format!("{selected}:{item}")),
            "false:c\ntrue:d"
        );

        let Element::Box(box_el) = list.element::<()>() else {
            panic!("expected box element");
        };
        assert_eq!(box_el.children.len(), 2);
        let Element::Text(last_item) = box_el.children.last().expect("expected last item") else {
            panic!("expected list item");
        };
        assert_eq!(last_item.content, "▸ d");
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
