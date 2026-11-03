use super::StateError;
use alloc::vec::Vec;
pub struct History<T> {
    entries: Vec<T>,
    cursor: usize,
    capacity: usize,
    grouping: bool,
    pending: Option<T>,
}
impl<T: Clone + PartialEq> History<T> {
    pub fn new(initial: T, capacity: usize) -> Result<Self, StateError> {
        if !(2..=1024).contains(&capacity) {
            return Err(StateError::Limit);
        }
        let mut entries = Vec::with_capacity(capacity);
        entries.push(initial);
        Ok(Self {
            entries,
            cursor: 0,
            capacity,
            grouping: false,
            pending: None,
        })
    }
    pub fn current(&self) -> &T {
        &self.entries[self.cursor]
    }
    pub fn is_grouping(&self) -> bool {
        self.grouping
    }
    pub fn can_undo(&self) -> bool {
        !self.grouping && self.cursor > 0
    }
    pub fn can_redo(&self) -> bool {
        !self.grouping && self.cursor + 1 < self.entries.len()
    }
    pub fn undo_target(&self) -> Option<&T> {
        self.can_undo().then(|| &self.entries[self.cursor - 1])
    }
    pub fn redo_target(&self) -> Option<&T> {
        self.can_redo().then(|| &self.entries[self.cursor + 1])
    }
    pub fn record(&mut self, value: T) -> bool {
        if self.grouping {
            self.pending = Some(value);
            return false;
        }
        if self.current() == &value {
            return false;
        }
        self.entries.truncate(self.cursor + 1);
        if self.entries.len() == self.capacity {
            self.entries.remove(0);
        }
        self.entries.push(value);
        self.cursor = self.entries.len() - 1;
        true
    }
    pub fn undo(&mut self) -> Option<&T> {
        if !self.can_undo() {
            return None;
        }
        self.cursor -= 1;
        Some(self.current())
    }
    pub fn redo(&mut self) -> Option<&T> {
        if !self.can_redo() {
            return None;
        }
        self.cursor += 1;
        Some(self.current())
    }
    pub fn begin_group(&mut self) {
        assert!(!self.grouping, "history groups cannot nest");
        self.grouping = true;
    }
    pub fn end_group(&mut self) -> bool {
        assert!(self.grouping, "no history group");
        self.grouping = false;
        self.pending.take().is_some_and(|value| self.record(value))
    }
    pub fn cancel_group(&mut self) {
        self.grouping = false;
        self.pending = None;
    }
    pub fn reset(&mut self, value: T) {
        self.entries.clear();
        self.entries.push(value);
        self.cursor = 0;
        self.grouping = false;
        self.pending = None;
    }
}
