use std::mem;

#[derive(Debug, Clone)]
pub struct History<T> {
    past: Vec<T>,
    present: T,
    future: Vec<T>,
}

impl<T> History<T> {
    pub fn new(present: T) -> Self {
        Self {
            past: Vec::new(),
            present,
            future: Vec::new(),
        }
    }

    pub fn present(&self) -> &T {
        &self.present
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    pub fn commit(&mut self, value: T) {
        self.past.push(mem::replace(&mut self.present, value));
        self.future.clear();
    }

    pub fn undo(&mut self) -> bool {
        step(&mut self.past, &mut self.present, &mut self.future)
    }

    pub fn redo(&mut self) -> bool {
        step(&mut self.future, &mut self.present, &mut self.past)
    }
}

fn step<T>(from: &mut Vec<T>, present: &mut T, to: &mut Vec<T>) -> bool {
    let Some(value) = from.pop() else {
        return false;
    };
    to.push(mem::replace(present, value));
    true
}

#[cfg(test)]
mod tests {
    use crate::history::History;

    #[test]
    fn starts_with_the_first_value_and_nothing_to_undo_or_redo() {
        let history = History::new(1);
        assert_eq!(*history.present(), 1);
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn a_commit_makes_the_new_value_the_present() {
        let mut history = History::new(1);
        history.commit(2);
        assert_eq!(*history.present(), 2);
        assert!(history.can_undo());
    }

    #[test]
    fn undo_restores_the_values_in_reverse_order() {
        let mut history = History::new(1);
        history.commit(2);
        history.commit(3);
        assert!(history.undo());
        assert_eq!(*history.present(), 2);
        assert!(history.undo());
        assert_eq!(*history.present(), 1);
    }

    #[test]
    fn undo_with_nothing_to_undo_changes_nothing() {
        let mut history = History::new(1);
        assert!(!history.undo());
        assert_eq!(*history.present(), 1);
    }

    #[test]
    fn redo_restores_the_value_that_undo_removed() {
        let mut history = History::new(1);
        history.commit(2);
        history.undo();
        assert!(history.can_redo());
        assert!(history.redo());
        assert_eq!(*history.present(), 2);
    }

    #[test]
    fn redo_with_nothing_to_redo_changes_nothing() {
        let mut history = History::new(1);
        history.commit(2);
        assert!(!history.redo());
        assert_eq!(*history.present(), 2);
    }

    #[test]
    fn a_commit_after_an_undo_removes_the_values_to_redo() {
        let mut history = History::new(1);
        history.commit(2);
        history.undo();
        history.commit(3);
        assert!(!history.can_redo());
        assert_eq!(*history.present(), 3);
        assert!(history.undo());
        assert_eq!(*history.present(), 1);
    }
}
