//! Focus management for navigating between interactive components.

/// Unique identifier for a focusable element.
pub type FocusId = u32;

/// Manages focus state across multiple focusable elements.
pub struct FocusManager {
    focusable: Vec<FocusId>,
    current: Option<usize>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            focusable: Vec::new(),
            current: None,
        }
    }

    pub fn register(&mut self, id: FocusId) {
        if !self.focusable.contains(&id) {
            self.focusable.push(id);
            if self.current.is_none() {
                self.current = Some(0);
            }
        }
    }

    pub fn unregister(&mut self, id: FocusId) {
        if let Some(pos) = self.focusable.iter().position(|&x| x == id) {
            self.focusable.remove(pos);
            if self.focusable.is_empty() {
                self.current = None;
            } else if let Some(cur) = self.current {
                if cur >= self.focusable.len() {
                    self.current = Some(self.focusable.len() - 1);
                }
            }
        }
    }

    pub fn focus_next(&mut self) {
        if self.focusable.is_empty() {
            return;
        }
        self.current = Some(match self.current {
            Some(idx) => (idx + 1) % self.focusable.len(),
            None => 0,
        });
    }

    pub fn focus_prev(&mut self) {
        if self.focusable.is_empty() {
            return;
        }
        self.current = Some(match self.current {
            Some(0) => self.focusable.len() - 1,
            Some(idx) => idx - 1,
            None => 0,
        });
    }

    pub fn focus(&mut self, id: FocusId) {
        if let Some(pos) = self.focusable.iter().position(|&x| x == id) {
            self.current = Some(pos);
        }
    }

    pub fn is_focused(&self, id: FocusId) -> bool {
        match self.current {
            Some(idx) => self.focusable.get(idx) == Some(&id),
            None => false,
        }
    }

    pub fn current(&self) -> Option<FocusId> {
        self.current
            .and_then(|idx| self.focusable.get(idx).copied())
    }

    pub fn clear(&mut self) {
        self.focusable.clear();
        self.current = None;
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_sets_initial_focus() {
        let mut fm = FocusManager::new();
        fm.register(1);
        assert_eq!(fm.current(), Some(1));
        assert!(fm.is_focused(1));
    }

    #[test]
    fn focus_next_cycles() {
        let mut fm = FocusManager::new();
        fm.register(1);
        fm.register(2);
        fm.register(3);
        assert_eq!(fm.current(), Some(1));
        fm.focus_next();
        assert_eq!(fm.current(), Some(2));
        fm.focus_next();
        assert_eq!(fm.current(), Some(3));
        fm.focus_next();
        assert_eq!(fm.current(), Some(1));
    }

    #[test]
    fn focus_prev_cycles() {
        let mut fm = FocusManager::new();
        fm.register(1);
        fm.register(2);
        fm.register(3);
        fm.focus_prev();
        assert_eq!(fm.current(), Some(3));
        fm.focus_prev();
        assert_eq!(fm.current(), Some(2));
    }

    #[test]
    fn unregister_adjusts_focus() {
        let mut fm = FocusManager::new();
        fm.register(1);
        fm.register(2);
        fm.register(3);
        fm.focus_next();
        fm.focus_next();
        assert_eq!(fm.current(), Some(3));
        fm.unregister(3);
        assert_eq!(fm.current(), Some(2));
    }

    #[test]
    fn unregister_all_clears_focus() {
        let mut fm = FocusManager::new();
        fm.register(1);
        fm.unregister(1);
        assert_eq!(fm.current(), None);
    }

    #[test]
    fn focus_specific_id() {
        let mut fm = FocusManager::new();
        fm.register(10);
        fm.register(20);
        fm.register(30);
        fm.focus(20);
        assert!(fm.is_focused(20));
        assert!(!fm.is_focused(10));
    }

    #[test]
    fn clear_removes_all() {
        let mut fm = FocusManager::new();
        fm.register(1);
        fm.register(2);
        fm.clear();
        assert_eq!(fm.current(), None);
    }

    #[test]
    fn focus_next_on_empty_is_noop() {
        let mut fm = FocusManager::new();
        fm.focus_next();
        assert_eq!(fm.current(), None);
    }
}
