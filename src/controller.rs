use crate::{
    model::{Caret, Model},
    utils::{read_from_clipboard, save_to_clipboard},
    view::*,
    vim::*,
    UsizeMax,
};
use std::convert::TryFrom;
use std::mem::swap;
use termion::{self, event::Key};

#[derive(Clone, Debug)]
pub enum Msg {
    Byte(u8),
    Move(Direction),
    Quit,
    QuitWithoutSaving,
    Save,
    SaveAs(String),
    SaveAndQuit,
    Switch(Option<InputMode>),
    Delete(Option<Movement>),
    ToNormal,
    ToInsert(Option<usize>),
    ToAppend(Option<usize>),
    ToReplace,
    ToVisual,
    ToCommand,
    ClipboardCopy,
    ClipboardPaste,
    Yank,
    Paste(Option<Movement>),
    Undo,
    Redo,
    Show(String),
    Redraw,
    Resize((u16, u16)),
}

#[derive(Clone, Debug)]
pub enum Movement {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
    //Start,
    Offset(usize),
    //End,
    Newline,
    Revert,
}

impl TryFrom<Key> for Direction {
    type Error = String;

    fn try_from(value: Key) -> Result<Self, Self::Error> {
        use Key::*;
        match value {
            Left | Char('h') => Ok(Direction::Left),
            Right | Char('l') => Ok(Direction::Right),
            Up | Char('k') => Ok(Direction::Up),
            Down | Char('j') => Ok(Direction::Down),
            _ => Err(format!("Key {:?} can't be converted to a Direction", value)),
        }
    }
}

pub struct Controller {
    pub state: VimState,
    pub model: Model,
    pub view: View,
    mode: InputMode,
    yank: Option<Vec<u8>>,
}

impl Controller {
    pub fn new(model: Model, view: View) -> Controller {
        Controller {
            state: VimState::Normal,
            model: model,
            view: view,
            mode: InputMode::Hex,
            yank: None,
        }
    }

    // Opening, Saving, etc.

    pub fn open(&mut self, path: &str) {
        match self.model.open(path) {
            Ok(_) => self.view.status_view.set_head(path),
            Err(e) => self.view.status_view.set_head(&format!("error: {}", e)),
        }
    }

    pub fn save(&mut self) -> bool {
        match self.model.save() {
            Ok(_) => {
                self.view
                    .status_view
                    .set_body(&format!("\"{}\" saved", self.model.path));
                true
            }
            Err(error) => {
                self.view.status_view.set_body(&format!(
                    "could not save \"{}\": {}",
                    self.model.path, error
                ));
                false
            }
        }
    }

    pub fn save_as(&mut self, path: String) -> bool {
        match self.model.save_as(&path) {
            Ok(_) => {
                self.view
                    .status_view
                    .set_body(&format!("\"{}\" saved", &path));
                true
            }
            Err(error) => {
                self.view.status_view.set_body(&format!(
                    "could not save \"{}\": {}",
                    self.model.path, error
                ));
                false
            }
        }
    }

    // Editing

    pub fn insert(&mut self, value: u8) {
        let index = self.model.get_index();
        if let Err(e) = self.model.edit(index, index, &[value]) {
            self.view
                .status_view
                .set_body(&format!("could not insert value ({})", e));
        }
        self.model.inc_index(1);
    }

    pub fn paste(&mut self, index: usize, value: &[u8]) {
        if let Err(e) = self.model.edit(index, index, value) {
            self.view
                .status_view
                .set_body(&format!("could not insert value ({})", e));
        }
        self.model.inc_index(value.len());
        self.view.hex_view.scroll_to(self.model.get_index());
    }

    pub fn remove_left(&mut self) {
        let index = self.model.get_index();

        // If cursor is at end, deleting will move the cursor to the left since the range updates automatically...
        let end = index == self.model.buffer.len();

        if let Err(e) = self.model.edit(index.saturating_sub(1), index, &[]) {
            self.view
                .status_view
                .set_body(&format!("could not remove value ({})", e));
        }

        // ...thus do not move again. (TODO: Refactor)
        if !end {
            self.model.dec_index(1);
        }
    }

    pub fn remove_right(&mut self) {
        let index = self.model.get_index();
        if let Err(e) = self.model.edit(index, index.saturating_add(1), &[]) {
            self.view
                .status_view
                .set_body(&format!("could not remove value ({})", e));
        }
    }

    pub fn replace(&mut self, value: u8) {
        let index = self.model.get_index();
        if let Err(e) = self.model.edit(index, index.saturating_add(1), &[value]) {
            self.view
                .status_view
                .set_body(&format!("could not replace value ({})", e));
        }
    }

    // Update

    pub fn update(&mut self, msg: Msg) -> bool {
        let mut run = true;

        match msg {
            Msg::Byte(byte) => {
                match self.model.caret {
                    Caret::Index(_) => {
                        self.insert(byte);
                    }
                    Caret::Replace(_) => {
                        self.replace(byte);
                    }
                    _ => {}
                }

                self.model.snapshot();
            }
            Msg::Move(dir) => {
                match dir {
                    Direction::Left => self.model.dec_index(1),
                    Direction::Right => self.model.inc_index(1),
                    Direction::Up => self.model.dec_index(16),
                    Direction::Down => self.model.inc_index(16),
                    Direction::Offset(offset) => {
                        self.model.set_index(offset);
                        self.view.hex_view.scroll_to(self.model.get_index());

                        let index = match self.model.caret {
                            Caret::Index(index)
                            | Caret::Offset(index)
                            | Caret::Replace(index)
                            | Caret::Visual(_, index) => index,
                        };

                        self.view.status_view.set_index(index.into());
                        self.view.status_view.set_body("");
                    }
                    Direction::Newline => {
                        self.model.inc_index(16);
                        let index = self.model.get_index();
                        self.model.set_index(index - (index % 16));
                    }
                    Direction::Revert => {
                        if let Caret::Visual(ref mut start, ref mut end) = self.model.caret {
                            swap(start, end);
                        } else {
                            return true;
                        }
                    }
                };

                self.view.hex_view.scroll_to(self.model.get_index());
                self.view.status_view.set_index(self.model.get_index());
            }
            Msg::Quit => {
                if self.model.is_modified() {
                    self.view
                        .status_view
                        .set_body("save your changes with :w or force quit with :q!");
                } else {
                    run = false;
                }
            }
            Msg::QuitWithoutSaving => {
                run = false;
            }
            Msg::Save => {
                self.save();
            }
            Msg::SaveAs(path) => {
                self.save_as(path);
            }
            Msg::SaveAndQuit => {
                if self.save() {
                    run = false;
                }
            }
            Msg::Switch(mode) => match mode {
                Some(InputMode::Ascii) => {
                    self.mode = InputMode::Ascii;
                    self.view.status_view.set_body(&format!(
                        "{}-- Normal (Ascii) --{}",
                        termion::style::Bold,
                        termion::style::Reset
                    ));
                }
                Some(InputMode::Hex) => {
                    self.mode = InputMode::Hex;
                    self.view.status_view.set_body(&format!(
                        "{}-- Normal (Hex) --{}",
                        termion::style::Bold,
                        termion::style::Reset
                    ));
                }
                None => {
                    self.mode = match self.mode {
                        InputMode::Hex => {
                            self.view.status_view.set_body(&format!(
                                "{}-- Normal (Ascii) --{}",
                                termion::style::Bold,
                                termion::style::Reset
                            ));
                            InputMode::Ascii
                        }
                        InputMode::Ascii => {
                            self.view.status_view.set_body(&format!(
                                "{}-- Normal (Hex) --{}",
                                termion::style::Bold,
                                termion::style::Reset
                            ));
                            InputMode::Hex
                        }
                    };
                }
            },
            Msg::Delete(movement) => {
                if self.model.buffer.is_empty() {
                    return true;
                }

                match movement {
                    Some(Movement::Left) => {
                        self.remove_left();
                        self.model.snapshot();
                    }
                    Some(Movement::Right) => {
                        if let Caret::Offset(_) = self.model.caret {
                            self.yank = Some(
                                self.model.buffer
                                    [self.model.get_index()..self.model.get_index() + 1]
                                    .to_owned(),
                            );
                        }
                        self.remove_right();
                        self.model.snapshot();
                    }
                    None => {
                        if let Caret::Visual(start, end) = self.model.caret {
                            let (start, end) = if usize::from(start) > usize::from(end) {
                                (end, start)
                            } else {
                                (start, end)
                            };

                            self.yank = Some(
                                self.model.buffer[start.into()..usize::from(end) + 1].to_owned(),
                            );

                            if let Err(e) = self.model.edit(start.into(), usize::from(end) + 1, &[])
                            {
                                self.view
                                    .status_view
                                    .set_body(&format!("could not remove range ({})", e));
                            } else {
                                self.model.set_index(start.into());
                            }

                            self.view.hex_view.scroll_to(self.model.get_index());

                            self.model.snapshot();
                        }
                    }
                }
            }
            Msg::ToNormal => {
                self.model.caret = match self.model.caret {
                    Caret::Index(index) => Caret::Offset(UsizeMax::new(
                        index.value.saturating_sub(1),
                        index.get_maximum().saturating_sub(1),
                    )),
                    Caret::Offset(index) | Caret::Replace(index) | Caret::Visual(_, index) => {
                        Caret::Offset(index)
                    }
                };

                self.view.status_view.set_body(&format!(
                    "{}-- Normal ({:?}) --{}",
                    termion::style::Bold,
                    self.mode,
                    termion::style::Reset
                )); // TODO
            }
            Msg::ToInsert(_repeat) => {
                self.model.caret = match self.model.caret {
                    Caret::Index(index) => Caret::Index(index),
                    Caret::Offset(index) | Caret::Replace(index) | Caret::Visual(_, index) => Caret::Index(
                        UsizeMax::new(index.value, index.get_maximum().saturating_add(1)),
                    ),
                };

                self.view.status_view.set_body(&format!(
                    "{}-- Insert ({:?}) --{}",
                    termion::style::Bold,
                    self.mode,
                    termion::style::Reset
                )); // TODO
            }
            Msg::ToAppend(_repeat) => {
                self.model.caret = match self.model.caret {
                    Caret::Index(index) => Caret::Index(index),
                    Caret::Offset(index) | Caret::Replace(index) | Caret::Visual(_, index) => Caret::Index(
                        UsizeMax::new(index.value, index.get_maximum().saturating_add(1)),
                    ),
                };

                self.view.status_view.set_body(&format!(
                    "{}-- Insert ({:?}) --{}",
                    termion::style::Bold,
                    self.mode,
                    termion::style::Reset
                )); // TODO

                self.update(Msg::Move(Direction::Right));
            }
            Msg::ToReplace => {
                self.model.caret = match self.model.caret {
                    Caret::Index(index) => Caret::Replace(UsizeMax::new(
                        index.value,
                        index.get_maximum().saturating_sub(1),
                    )),
                    Caret::Offset(index) | Caret::Replace(index) | Caret::Visual(_, index) => {
                        Caret::Replace(index)
                    }
                };

                self.view.status_view.set_body(&format!(
                    "{}-- Replace ({:?}) --{}",
                    termion::style::Bold,
                    self.mode,
                    termion::style::Reset
                )); // TODO
            }
            Msg::ToVisual => {
                self.model.caret = match self.model.caret {
                    Caret::Index(index) => Caret::Visual(
                        UsizeMax::new(index.value, index.get_maximum().saturating_sub(1)),
                        UsizeMax::new(index.value, index.get_maximum().saturating_sub(1)),
                    ),
                    Caret::Offset(index) | Caret::Replace(index) => Caret::Visual(index, index),
                    Caret::Visual(start, begin) => Caret::Visual(start, begin),
                };

                self.view.status_view.set_body(&format!(
                    "{}-- Visual --{}",
                    termion::style::Bold,
                    termion::style::Reset
                ));
            }
            Msg::ToCommand => {
                self.view.status_view.set_body(":");
            }
            Msg::ClipboardCopy => {
                if self.model.buffer.is_empty() {
                    return true;
                }

                let bytes = match self.model.caret {
                    Caret::Offset(index) => &self.model.buffer[index.value..index.value + 1],
                    Caret::Visual(start, end) => {
                        let (start, end) = if usize::from(start) > usize::from(end) {
                            (end, start)
                        } else {
                            (start, end)
                        };

                        &self.model.buffer[start.into()..usize::from(end) + 1]
                    }
                    _ => return true,
                };

                match save_to_clipboard(bytes) {
                    Ok(msg) | Err(msg) => self.view.status_view.set_body(&msg),
                };
            }
            Msg::ClipboardPaste => match read_from_clipboard() {
                Ok(value) => {
                    let index = self.model.get_index();
                    self.paste(index, &value);
                    self.model.snapshot();
                }
                Err(ref e) => {
                    self.view.status_view.set_body(e);
                }
            },
            Msg::Yank => {
                if self.model.buffer.is_empty() {
                    return true;
                }

                match self.model.caret {
                    Caret::Offset(index) => {
                        self.yank = Some(vec![self.model.buffer[index.value]]);
                    }
                    Caret::Visual(start, end) => {
                        let (start, end) = if usize::from(start) > usize::from(end) {
                            (end, start)
                        } else {
                            (start, end)
                        };

                        self.yank =
                            Some(self.model.buffer[start.into()..usize::from(end) + 1].to_owned());
                        self.update(Msg::ToNormal);
                    }
                    _ => return true,
                }
            }
            Msg::Paste(movement) => {
                if let Some(value) = self.yank.clone() {
                    match movement {
                        Some(Movement::Left) | None => {
                            let index = self.model.get_index();
                            self.paste(index, &value);
                            self.update(Msg::Move(Direction::Left));
                            self.model.snapshot();
                        }
                        Some(Movement::Right) => {
                            let index = self.model.get_index() + 1;
                            self.paste(index, &value);
                            self.model.snapshot();
                        }
                    }
                }
            }
            Msg::Undo => {
                if !self.model.undo() {
                    self.view.status_view.set_body("Nothing to undo");
                }
                self.view.hex_view.scroll_to(self.model.get_index());
            }
            Msg::Redo => {
                if !self.model.redo() {
                    self.view.status_view.set_body("Nothing to redo");
                }
                self.view.hex_view.scroll_to(self.model.get_index());
            }
            Msg::Show(msg) => {
                self.view.status_view.set_body(&format!("{}", &msg));
            }
            Msg::Redraw => {
                if let Err(error) = self.view.draw(&self.model) {
                    // What to do when drawing failed?
                    // Try to report this on stderr and ignore further failures.
                    eprintln!("{}", error);
                }
            }
            Msg::Resize(size) => {
                self.view.set_area(DrawArea {
                    origin: (1, 1),
                    dimens: size,
                });
            }
        };

        run
    }

    // Transitions

    // TODO: Refactor into VimStateMachine
    pub fn transition(&mut self, key: Key) -> bool {
        use termion::event::Key::{
            Alt, Backspace, Char, Ctrl, Delete, Down, Esc, Insert, Left, Right, Up,
        };

        // TODO: Quickfix for tmux
        let key = if key == Alt('\u{1b}') { Esc } else { key };

        let mut run = true;

        self.state = match self.state.clone() {
            VimState::Normal => match key {
                Left | Right | Up | Down | Char('h') | Char('l') | Char('k') | Char('j') => {
                    self.update(Msg::Move(Direction::try_from(key).unwrap()));
                    VimState::Normal
                }
                Backspace => {
                    self.update(Msg::Move(Direction::Left));
                    VimState::Normal
                }
                Char('\t') => {
                    self.update(Msg::Switch(None));
                    VimState::Normal
                }
                Char('a') => {
                    self.update(Msg::ToAppend(None));
                    VimState::Insert(InputStateMachine::new(self.mode))
                }
                Char('i') => {
                    self.update(Msg::ToInsert(None));
                    VimState::Insert(InputStateMachine::new(self.mode))
                }
                Delete | Char('x') => {
                    self.update(Msg::Delete(Some(Movement::Right)));
                    VimState::Normal
                }
                Char('r') => {
                    self.update(Msg::ToReplace);
                    VimState::Replace(InputStateMachine::new(self.mode), false)
                }
                Char('R') => {
                    self.update(Msg::ToReplace);
                    VimState::Replace(InputStateMachine::new(self.mode), true)
                }
                Char('v') => {
                    self.update(Msg::ToVisual);
                    VimState::Visual
                }
                Char(':') => {
                    self.update(Msg::ToCommand);
                    VimState::Command(String::new())
                }
                Char('\n') => {
                    self.update(Msg::Move(Direction::Newline));
                    VimState::Normal
                }
                Ctrl('c') => {
                    self.update(Msg::ClipboardCopy);
                    VimState::Normal
                }
                Char('y') => {
                    self.update(Msg::Yank);
                    VimState::Normal
                }
                Char('p') => {
                    self.update(Msg::Paste(Some(Movement::Right)));
                    VimState::Normal
                }
                Char('P') => {
                    self.update(Msg::Paste(Some(Movement::Left)));
                    VimState::Normal
                }
                Char('u') => {
                    self.update(Msg::Undo);
                    VimState::Normal
                }
                Ctrl('r') => {
                    self.update(Msg::Redo);
                    VimState::Normal
                }
                Esc => {
                    self.update(Msg::ToNormal);
                    VimState::Normal
                }
                _ => VimState::Normal,
            },
            VimState::Insert(mut machine) => {
                if machine.initial() {
                    match key {
                        Left | Right | Up | Down => {
                            self.update(Msg::Move(Direction::try_from(key).unwrap()));
                            VimState::Insert(machine)
                        }
                        Backspace => {
                            self.update(Msg::Delete(Some(Movement::Left)));
                            VimState::Insert(machine)
                        }
                        Delete => {
                            self.update(Msg::Delete(Some(Movement::Right)));
                            VimState::Insert(machine)
                        }
                        Insert => {
                            self.update(Msg::ToReplace);
                            VimState::Replace(InputStateMachine::new(self.mode), true)
                        }
                        Char(a) if machine.valid_input(a) => {
                            machine.transition(key);
                            match machine.state.clone() {
                                InputState::Done(byte) => {
                                    self.update(Msg::Byte(byte));
                                    VimState::Insert(InputStateMachine::new(self.mode))
                                }
                                InputState::Incomplete(_) => VimState::Insert(machine),
                            }
                        }
                        Char('\t') => {
                            self.update(Msg::Switch(None));
                            VimState::Insert(InputStateMachine::new(self.mode))
                        }
                        Ctrl('v') => {
                            self.update(Msg::ClipboardPaste);
                            VimState::Insert(machine)
                        }
                        Esc => {
                            self.update(Msg::ToNormal);
                            VimState::Normal
                        }
                        _ => VimState::Insert(machine),
                    }
                } else {
                    match key {
                        Char(a) if machine.valid_input(a) => {
                            machine.transition(key);
                            match machine.state.clone() {
                                InputState::Done(byte) => {
                                    self.update(Msg::Byte(byte));
                                    VimState::Insert(InputStateMachine::new(self.mode))
                                }
                                InputState::Incomplete(_) => VimState::Insert(machine),
                            }
                        }
                        Esc => {
                            self.update(Msg::ToNormal);
                            VimState::Normal
                        }
                        _ => VimState::Insert(machine),
                    }
                }
            }
            VimState::Replace(mut machine, many) => {
                if machine.initial() {
                    match key {
                        Left | Right | Up | Down | Char('h') | Char('l') | Char('k')
                        | Char('j')
                            if many =>
                        {
                            self.update(Msg::Move(Direction::try_from(key).unwrap()));
                            VimState::Replace(machine, many)
                        }
                        Backspace => {
                            self.update(Msg::Move(Direction::Left));
                            VimState::Replace(machine, many)
                        }
                        Char(c) if machine.valid_input(c) => {
                            machine.transition(key);
                            match machine.state.clone() {
                                InputState::Done(byte) => {
                                    self.update(Msg::Byte(byte));
                                    if many {
                                        self.update(Msg::Move(Direction::Right));
                                        VimState::Replace(InputStateMachine::new(self.mode), many)
                                    } else {
                                        self.update(Msg::ToNormal);
                                        VimState::Normal
                                    }
                                }
                                InputState::Incomplete(_) => VimState::Replace(machine, many),
                            }
                        }
                        Char('\t') => {
                            self.update(Msg::Switch(None));
                            VimState::Replace(InputStateMachine::new(self.mode), many)
                        }
                        Esc => {
                            self.update(Msg::ToNormal);
                            VimState::Normal
                        }
                        _ => VimState::Replace(machine, many),
                    }
                } else {
                    match key {
                        Char(c) if machine.valid_input(c) => {
                            machine.transition(key);
                            match machine.state.clone() {
                                InputState::Done(byte) => {
                                    self.update(Msg::Byte(byte));
                                    if many {
                                        self.update(Msg::Move(Direction::Right));
                                        VimState::Replace(InputStateMachine::new(self.mode), many)
                                    } else {
                                        self.update(Msg::ToNormal);
                                        VimState::Normal
                                    }
                                }
                                InputState::Incomplete(_) => VimState::Replace(machine, many),
                            }
                        }
                        Esc => {
                            self.update(Msg::ToNormal);
                            VimState::Normal
                        }
                        _ => VimState::Replace(machine, many),
                    }
                }
            }
            VimState::Visual => match key {
                Left | Right | Up | Down | Char('h') | Char('l') | Char('k') | Char('j') => {
                    self.update(Msg::Move(Direction::try_from(key).unwrap()));
                    VimState::Visual
                }
                Char('y') => {
                    self.update(Msg::Yank);
                    VimState::Normal
                }
                Ctrl('c') => {
                    self.update(Msg::ClipboardCopy);
                    VimState::Visual
                }
                Char('o') => {
                    self.update(Msg::Move(Direction::Revert));
                    VimState::Visual
                }
                Char('x') | Char('d') => {
                    self.update(Msg::Delete(None));
                    self.update(Msg::ToNormal);
                    VimState::Normal
                }
                Esc => {
                    self.update(Msg::ToNormal);
                    VimState::Normal
                }
                _ => VimState::Visual,
            },
            VimState::Command(mut cmd) => match key {
                Char('\n') => {
                    match Msg::parse(&cmd) {
                        Ok(cmd) => run = self.update(cmd),
                        Err(msg) => {
                            self.update(Msg::Show(format!("{}", &msg)));
                        }
                    }
                    VimState::Normal
                }
                Backspace => {
                    cmd.pop();
                    self.update(Msg::Show(format!(":{}", &cmd)));
                    VimState::Command(cmd)
                }
                Char(c) => {
                    cmd.push(c);
                    self.update(Msg::Show(format!(":{}", &cmd)));
                    VimState::Command(cmd)
                }
                Esc => {
                    self.update(Msg::Show("".into()));
                    VimState::Normal
                }
                _ => VimState::Command(cmd),
            },
        };

        run
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{quickcheck, Arbitrary, Gen};
    use std::{cell::RefCell, io::stdout, rc::Rc};
    use termion::{raw::IntoRawMode, screen::AlternateScreen};

    // TODO: `impl Arbitrary`'s are error-prone: new variants are easily missed. Better idea?

    impl Arbitrary for Msg {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use Msg::*;
            match g.next_u32() % 19 {
                0 => Byte(u8::arbitrary(g)),
                1 => Move(Direction::arbitrary(g)),
                //0 => Quit,
                //0 => QuitWithoutSaving,
                //0 => Save,
                //0 => SaveAs(String),
                //0 => SaveAndQuit,
                2 => Switch(Option::<InputMode>::arbitrary(g)),
                3 => Delete(Option::<Movement>::arbitrary(g)),
                4 => ToNormal,
                5 => ToInsert(Option::<usize>::arbitrary(g)),
                6 => ToAppend(Option::<usize>::arbitrary(g)),
                7 => ToReplace,
                8 => ToVisual,
                9 => ToCommand,
                10 => ClipboardCopy,
                11 => ClipboardPaste,
                12 => Yank,
                13 => Paste(Option::<Movement>::arbitrary(g)),
                14 => Undo,
                15 => Redo,
                16 => Show(String::arbitrary(g)),
                17 => Redraw,
                18 => Resize(<(u16, u16)>::arbitrary(g)),
                _ => panic!(),
            }
        }
    }

    impl Arbitrary for Movement {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use Movement::*;
            match g.next_u32() % 2 {
                0 => Left,
                1 => Right,
                _ => panic!(),
            }
        }
    }

    impl Arbitrary for Direction {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use Direction::*;
            match g.next_u32() % 7 {
                0 => Left,
                1 => Right,
                2 => Up,
                3 => Down,
                //Start,
                4 => Offset(usize::arbitrary(g)),
                //End,
                5 => Newline,
                6 => Revert,
                _ => panic!(),
            }
        }
    }

    impl Arbitrary for InputMode {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use InputMode::*;
            match g.next_u32() % 2 {
                0 => Ascii,
                1 => Hex,
                _ => panic!(),
            }
        }
    }

    quickcheck! {
        fn test_update(msgs: Vec<Msg>) -> bool {
            let stdout = Rc::new(RefCell::new(AlternateScreen::from(
                stdout().into_raw_mode().unwrap(),
            )));

            let mut ctrl = Controller::new(Model::new(), View::new(stdout));

            for msg in msgs {
                ctrl.update(msg);
            }
            true
        }
    }
}
