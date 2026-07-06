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

    /// Select the first item when one exists.
    fn select_first(&mut self) {
        if !self.is_empty() {
            self.select_index(0);
        }
    }

    /// Select the last item when one exists.
    fn select_last(&mut self) {
        if let Some(last) = self.item_count().checked_sub(1) {
            self.select_index(last);
        }
    }

    /// Move selection one item forward, clamping at the end.
    fn select_next(&mut self) {
        match self.selected_index() {
            Some(index) => self.select_index(index.saturating_add(1)),
            None => self.select_first(),
        }
    }

    /// Move selection one item backward, clamping at the beginning.
    fn select_previous(&mut self) {
        match self.selected_index() {
            Some(index) => self.select_index(index.saturating_sub(1)),
            None => self.select_first(),
        }
    }

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

    /// Move to the beginning of the scroll range.
    fn scroll_to_top(&mut self) {
        self.set_scroll_offset(0);
    }

    /// Move to the end of the component-owned scroll range.
    fn scroll_to_bottom(&mut self) {
        self.set_scroll_offset(usize::MAX);
    }

    /// Move the scroll offset by a signed delta, saturating at zero and letting
    /// the component clamp the upper bound.
    fn scroll_by(&mut self, delta: isize) {
        if delta < 0 {
            self.set_scroll_offset(self.scroll_offset().saturating_sub(delta.unsigned_abs()));
        } else {
            self.set_scroll_offset(self.scroll_offset().saturating_add(delta as usize));
        }
    }
}

/// A component with an active tab/source.
pub trait Tabbed {
    /// Number of tabs currently exposed by the component.
    fn tab_count(&self) -> usize;

    /// Current active tab, if any tabs exist.
    fn active_tab_index(&self) -> Option<usize>;

    /// Set the active tab by index, clamping to the component's valid range.
    fn set_active_tab_index(&mut self, index: usize);

    /// Switch to the first tab when one exists.
    fn set_first_tab(&mut self) {
        if self.tab_count() > 0 {
            self.set_active_tab_index(0);
        }
    }

    /// Switch to the last tab when one exists.
    fn set_last_tab(&mut self) {
        if let Some(last) = self.tab_count().checked_sub(1) {
            self.set_active_tab_index(last);
        }
    }

    /// Switch to the next tab, clamping at the end.
    fn set_next_tab(&mut self) {
        match self.active_tab_index() {
            Some(index) => self.set_active_tab_index(index.saturating_add(1)),
            None => self.set_first_tab(),
        }
    }

    /// Switch to the previous tab, clamping at the beginning.
    fn set_previous_tab(&mut self) {
        match self.active_tab_index() {
            Some(index) => self.set_active_tab_index(index.saturating_sub(1)),
            None => self.set_first_tab(),
        }
    }
}

/// A selectable component whose current item may or may not be activatable.
pub trait Activatable: Selectable {
    /// Whether the item at `index` exists and is disabled.
    fn is_item_disabled(&self, index: usize) -> bool;

    /// Whether the item at `index` exists and can currently emit an action.
    fn can_activate_index(&self, index: usize) -> bool {
        index < self.item_count() && !self.is_item_disabled(index)
    }

    /// Whether the current selected item can currently emit an action.
    fn can_activate_selected(&self) -> bool {
        self.selected_index()
            .is_some_and(|index| self.can_activate_index(index))
    }
}

#[cfg(test)]
mod tests {
    use super::{Activatable, Scrollable, Selectable, Tabbed};
    use crate::components::{
        ChoicePrompt, ChoicePromptItem, DataColumn, DataRow, DataTable, LevelSlider, List,
        MenuItem, MenuPanel, PreviewItem, PreviewPanel, TabbedMenuItem, TabbedMenuPanel,
        TabbedMenuTab, TreePicker, TreePickerItem,
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
    fn selectable_default_helpers_move_with_clamping() {
        let mut list = List::new(vec!["one", "two", "three"], 2);
        let mut slider = LevelSlider::from_labels(vec!["low", "medium", "high"]);
        let mut prompt = ChoicePrompt::new(
            "Pick",
            vec![
                ChoicePromptItem::new("one"),
                ChoicePromptItem::new("two"),
                ChoicePromptItem::new("three"),
            ],
        );

        list.select_next();
        slider.select_last();
        prompt.select_index(usize::MAX);

        assert_eq!(Selectable::selected_index(&list), Some(1));
        assert_eq!(Selectable::selected_index(&slider), Some(2));
        assert_eq!(Selectable::selected_index(&prompt), Some(2));

        list.select_previous();
        slider.select_next();
        prompt.select_first();

        assert_eq!(Selectable::selected_index(&list), Some(0));
        assert_eq!(Selectable::selected_index(&slider), Some(2));
        assert_eq!(Selectable::selected_index(&prompt), Some(0));
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

        menu.scroll_by(3);
        menu.scroll_by(-1);
        menu.scroll_to_top();
        menu.scroll_to_bottom();

        assert_eq!(Scrollable::scroll_offset(&menu), 0);
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

        panel.set_previous_tab();
        assert_eq!(panel.active_tab_index(), Some(0));
        panel.set_next_tab();
        assert_eq!(panel.active_tab_index(), Some(1));
        panel.set_first_tab();
        assert_eq!(panel.active_tab_index(), Some(0));
        panel.set_last_tab();
        assert_eq!(panel.active_tab_index(), Some(1));
    }

    #[test]
    fn activatable_trait_reports_disabled_selected_items() {
        let mut menu = MenuPanel::without_title().items(vec![
            MenuItem::new("enabled"),
            MenuItem::new("disabled").disabled(true),
        ]);
        let mut tree = TreePicker::without_title().items(vec![
            TreePickerItem::leaf("enabled"),
            TreePickerItem::leaf("disabled").disabled(true),
        ]);
        let mut preview = PreviewPanel::without_title().items(vec![
            PreviewItem::new("enabled"),
            PreviewItem::new("disabled").disabled(true),
        ]);
        let mut tabbed =
            TabbedMenuPanel::new(vec![TabbedMenuTab::new("One", Color::Cyan).items(vec![
                TabbedMenuItem::new("enabled"),
                TabbedMenuItem::new("disabled").disabled(true),
            ])]);
        let mut prompt = ChoicePrompt::new(
            "Pick",
            vec![
                ChoicePromptItem::new("enabled"),
                ChoicePromptItem::new("also enabled"),
            ],
        );

        menu.select_index(1);
        tree.select_index(1);
        preview.select_index(1);
        tabbed.select_index(1);
        prompt.select_index(1);

        assert!(!menu.can_activate_selected());
        assert!(!tree.can_activate_selected());
        assert!(!preview.can_activate_selected());
        assert!(!tabbed.can_activate_selected());
        assert!(prompt.can_activate_selected());

        assert!(menu.can_activate_index(0));
        assert!(!menu.can_activate_index(usize::MAX));
    }
}
