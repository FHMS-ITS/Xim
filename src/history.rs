#[derive(Debug)]
pub struct History<T> {
    done: Vec<T>,
    recall: Vec<T>,
}

impl<T: Clone> History<T> {
    pub fn new() -> History<T> {
        History {
            done: Vec::new(),
            recall: Vec::new(),
        }
    }

    pub fn init(&mut self, initial: &T) {
        self.snapshot(initial);
    }

    pub fn snapshot(&mut self, current: &T) {
        self.done.push(current.clone());
        self.recall.clear();
    }

    pub fn undo(&mut self) -> Option<T> {
        if self.done.len() <= 1 {
            return None;
        }

        if let Some(action) = self.done.pop() {
            self.recall.push(action);
            self.checkout()
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<T> {
        if let Some(action) = self.recall.pop() {
            self.done.push(action);
            self.checkout()
        } else {
            None
        }
    }

    pub fn checkout(&self) -> Option<T> {
        self.done.last().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::History;

    #[test]
    fn basic_undo_redo() {
        let mut hist: History<u8> = History::new();
        hist.init(&0);

        hist.snapshot(&1);
        hist.snapshot(&2);

        assert_eq!(hist.undo(), Some(1));
        assert_eq!(hist.undo(), Some(0));
        assert_eq!(hist.undo(), None);
        assert_eq!(hist.redo(), Some(1));
        assert_eq!(hist.redo(), Some(2));
        assert_eq!(hist.checkout(), Some(2));
        assert_eq!(hist.redo(), None);
        assert_eq!(hist.undo(), Some(1));
        assert_eq!(hist.undo(), Some(0));
        assert_eq!(hist.undo(), None);
        assert_eq!(hist.undo(), None);
        assert_eq!(hist.checkout(), Some(0));
        hist.snapshot(&3);
        assert_eq!(hist.undo(), Some(0));
        assert_eq!(hist.redo(), Some(3));
        assert_eq!(hist.checkout(), Some(3));
    }
}
