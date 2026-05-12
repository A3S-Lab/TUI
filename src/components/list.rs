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

    fn adjust_offset(&mut self) {
        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if self.cursor >= self.offset + self.height {
            self.offset = self.cursor - self.height + 1;
        }
    }
}
