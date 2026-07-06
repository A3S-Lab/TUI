//! Common state traits for interactive components.
//!
//! These traits are intentionally small. Components keep their own event
//! handlers and message enums, while app shells can use these contracts for
//! generic commands such as "move selection", "sync scroll", or "switch tab".

/// A component with a bounded selected item.
pub trait Selectable {
    /// Number of selectable rows/items currently exposed by the component.
    fn item_count(&self) -> usize;

    /// Current selected item, if the component has any selectable rows.
    fn selected_index(&self) -> Option<usize>;

    /// Select an item by index, clamping to the component's valid range.
    fn select_index(&mut self, index: usize);

    /// Whether this component currently has no selectable rows.
    fn is_empty(&self) -> bool {
        self.item_count() == 0
    }
}

/// A component with a scroll offset.
pub trait Scrollable {
    /// Current scroll offset.
    fn scroll_offset(&self) -> usize;

    /// Set the scroll offset, clamping when the component has a known bound.
    fn set_scroll_offset(&mut self, offset: usize);
}

/// A component with an active tab/source.
pub trait Tabbed {
    /// Number of tabs currently exposed by the component.
    fn tab_count(&self) -> usize;

    /// Current active tab, if any tabs exist.
    fn active_tab_index(&self) -> Option<usize>;

    /// Set the active tab by index, clamping to the component's valid range.
    fn set_active_tab_index(&mut self, index: usize);
}

#[cfg(test)]
mod tests {
    use super::{Scrollable, Selectable, Tabbed};
    use crate::components::{
        DataColumn, DataRow, DataTable, MenuItem, MenuPanel, PreviewItem, PreviewPanel,
        TabbedMenuItem, TabbedMenuPanel, TabbedMenuTab, TreePicker, TreePickerItem,
    };
    use crate::Color;

    fn select_last<T: Selectable>(component: &mut T) {
        component.select_index(usize::MAX);
    }

    fn push_scroll<T: Scrollable>(component: &mut T) {
        component.set_scroll_offset(usize::MAX);
    }

    #[test]
    fn selectable_trait_clamps_across_list_like_components() {
        let mut menu = MenuPanel::without_title().items(vec![
            MenuItem::new("one"),
            MenuItem::new("two"),
            MenuItem::new("three"),
        ]);
        let mut tree = TreePicker::without_title().items(vec![
            TreePickerItem::leaf("a.rs"),
            TreePickerItem::leaf("b.rs"),
        ]);
        let mut preview = PreviewPanel::without_title()
            .items(vec![PreviewItem::new("light"), PreviewItem::new("dark")]);
        let mut table = DataTable::new(vec![DataColumn::new("Name")])
            .row(DataRow::new(vec!["one"]))
            .row(DataRow::new(vec!["two"]));

        select_last(&mut menu);
        select_last(&mut tree);
        select_last(&mut preview);
        select_last(&mut table);

        assert_eq!(Selectable::selected_index(&menu), Some(2));
        assert_eq!(Selectable::selected_index(&tree), Some(1));
        assert_eq!(Selectable::selected_index(&preview), Some(1));
        assert_eq!(Selectable::selected_index(&table), Some(1));
    }

    #[test]
    fn scrollable_trait_clamps_across_list_like_components() {
        let mut menu = MenuPanel::without_title().items(vec![MenuItem::new("one")]);
        let mut tree = TreePicker::without_title().items(vec![TreePickerItem::leaf("a.rs")]);
        let mut preview = PreviewPanel::without_title().items(vec![PreviewItem::new("light")]);
        let mut table =
            DataTable::new(vec![DataColumn::new("Name")]).row(DataRow::new(vec!["one"]));

        push_scroll(&mut menu);
        push_scroll(&mut tree);
        push_scroll(&mut preview);
        push_scroll(&mut table);

        assert_eq!(Scrollable::scroll_offset(&menu), 0);
        assert_eq!(Scrollable::scroll_offset(&tree), 0);
        assert_eq!(Scrollable::scroll_offset(&preview), 0);
        assert_eq!(Scrollable::scroll_offset(&table), 0);
    }

    #[test]
    fn tabbed_trait_clamps_active_tab_and_resets_selection() {
        let mut panel = TabbedMenuPanel::new(vec![
            TabbedMenuTab::new("One", Color::Cyan).item(TabbedMenuItem::new("a")),
            TabbedMenuTab::new("Two", Color::Green)
                .items(vec![TabbedMenuItem::new("b"), TabbedMenuItem::new("c")]),
        ])
        .active_tab(0)
        .selected(usize::MAX);

        assert_eq!(Selectable::selected_index(&panel), Some(0));

        panel.set_active_tab_index(usize::MAX);

        assert_eq!(panel.tab_count(), 2);
        assert_eq!(panel.active_tab_index(), Some(1));
        assert_eq!(panel.item_count(), 2);
        assert_eq!(Selectable::selected_index(&panel), Some(0));
    }
}
