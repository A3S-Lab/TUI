pub type FocusId = u32;

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
        self.current.and_then(|idx| self.focusable.get(idx).copied())
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
