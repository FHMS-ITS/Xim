use super::{Caret, UsizeMax};
use super::Caret::*;

use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Result as IoResult};
use std::mem::swap;
use history::History;

#[derive(Debug)]
pub struct Model {
    pub path: String,
    pub caret: Caret,
    pub buffer: Vec<u8>,
    pub term_size: (u16, u16),
    history: History<(Vec<u8>, Caret)>,
}

impl Model {
    pub fn new() -> Model {
        Model {
            path: "".into(),
            caret: Caret::Offset(UsizeMax::new(0, 0)),
            buffer: vec![],
            term_size: (16, 16),
            history: History::new(),
        }
    }

    pub fn open(&mut self, path: &str) -> IoResult<()> {
        self.path = path.into();

        let buffer = {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(path)?;

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            buffer
        };

        self.buffer = buffer;
        self.caret = Caret::Offset(UsizeMax::new(0, self.buffer.len().saturating_sub(1)));

        self.history.init(&(self.buffer.clone(), self.caret.clone()));

        Ok(())
    }

    pub fn save(&self) -> IoResult<()> {
        let mut file = File::create(&self.path)?;
        file.write_all(&self.buffer)?;
        Ok(())
    }

    // FIXME: better be conservative first...
    pub fn is_modified(&self) -> bool {
        let disc_content = {
            let mut file = OpenOptions::new().create(false).read(true).write(false).open(&self.path).unwrap();
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).unwrap();
            buffer
        };

        self.buffer != disc_content
    }

    pub fn set_index(&mut self, new_index: usize) {
        match self.caret {
            Index(ref mut index) |
            Offset(ref mut index) |
            Replace(ref mut index) |
            Visual(_, ref mut index) => index.set_value(new_index),
        }
    }

    pub fn get_index(&self) -> usize {
        match self.caret {
            Index(index) |
            Offset(index) |
            Replace(index) |
            Visual(_, index) => index.into(),
        }
    }

    pub fn inc_index(&mut self, value: usize) {
        match self.caret {
            Index(ref mut index) |
            Offset(ref mut index) |
            Replace(ref mut index) |
            Visual(_, ref mut index) => *index += value,
        }
    }

    pub fn dec_index(&mut self, value: usize) {
        match self.caret {
            Index(ref mut index) |
            Offset(ref mut index) |
            Replace(ref mut index) |
            Visual(_, ref mut index) => *index -= value,
        }
    }

    pub fn snapshot(&mut self) {
        self.history.snapshot(&(self.buffer.clone(), self.caret.clone()));
    }

    pub fn undo(&mut self) -> bool {
        if let Some((older_buffer, older_caret)) = self.history.undo() {
            self.buffer = older_buffer;
            self.caret = older_caret;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some((newer_buffer, newer_caret)) = self.history.redo() {
            self.buffer = newer_buffer;
            self.caret = newer_caret;
            true
        } else {
            false
        }
    }

    pub fn edit(&mut self, mut start: usize, mut end: usize, new: &[u8]) -> Result<(), String> {
        if start > end {
            swap(&mut start, &mut end);
        }

        // Will eventually be replaced by ropes...
        if end <= self.buffer.len() {
            self.buffer.splice(start..end, new.iter().cloned());
        } else {
            return Err("no data to edit".into());
        }

        match self.caret {
            Index(ref mut index) => index.set_maximum(self.buffer.len()),
            Offset(ref mut index) |
            Replace(ref mut index) => index.set_maximum(self.buffer.len().saturating_sub(1)),
            Visual(ref mut start, ref mut end) => {
                start.set_maximum(self.buffer.len().saturating_sub(1));
                end.set_maximum(self.buffer.len().saturating_sub(1));
            }
        }

        Ok(())
    }

    pub fn set_term_size(&mut self, new_term_size: (u16, u16)) {
        self.term_size = new_term_size;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    quickcheck!{
        fn test_edit(buffer: Vec<u8>, start: usize, end: usize, new: Vec<u8>) -> bool {
            let mut buffer = buffer.clone();

            let mut model = Model {
                path: "".into(),
                caret: Caret::Offset(UsizeMax::new(0, buffer.len())),
                buffer: buffer.clone(),
                history: History::new(),
                term_size: (0, 0),
            };

            if start <= buffer.len() && end <= buffer.len() && start <= end {
                model.edit(start, end,  &new);
                buffer.splice(start..end, new.iter().cloned());
                buffer == model.buffer
            } else {
                true
            }
        }
    }
}
