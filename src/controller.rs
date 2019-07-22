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
    Move(Direction),
    Quit,
    QuitWithoutSaving,
    Save,
    SaveAs(String),
    SaveAndQuit,
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

    // Moving

    pub fn change_to_normal_mode(&mut self) {
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

    pub fn change_to_insert_mode(&mut self) {
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

    pub fn change_to_replace_mode(&mut self) {
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

    pub fn change_to_command_mode(&mut self) {
        self.view.status_view.set_body(":")
    }

    pub fn change_to_visual_mode(&mut self) {
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

    // Index

    pub fn set_index(&mut self, offset: usize) {
        self.model.set_index(offset);
        self.view.hex_view.scroll_to(self.model.get_index());

        let index = match self.model.caret {
            Caret::Index(index)
            | Caret::Offset(index)
            | Caret::Replace(index)
            | Caret::Visual(_, index) => index,
        };

        self.view.status_view.set_index(index.into());
    }

    pub fn set_index_aligned(&mut self) {
        let index = self.model.get_index();
        self.model.set_index(index - (index % 16));
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

    // Views

    pub fn resize_view(&mut self, size: (u16, u16)) {
        self.view.set_area(DrawArea {
            origin: (1, 1),
            dimens: size,
        });
    }

    pub fn update_view(&mut self) {
        if let Err(error) = self.view.draw(&self.model) {
            // What to do when drawing failed?
            // Try to report this on stderr and ignore further failures.
            eprintln!("{}", error);
        }
    }

    // Update

    fn update(&mut self, msg: Msg) -> bool {
        let mut run = true;
        match msg {
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
            Msg::Move(dir) => {
                match dir {
                    Direction::Left => self.model.dec_index(1),
                    Direction::Right => self.model.inc_index(1),
                    Direction::Up => self.model.dec_index(16),
                    Direction::Down => self.model.inc_index(16),
                    Direction::Offset(offset) => {
                        self.set_index(offset);
                        self.view.status_view.set_body("");
                    }
                };

                self.view.hex_view.scroll_to(self.model.get_index());
                self.view.status_view.set_index(self.model.get_index());
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
                    self.mode = match self.mode {
                        InputMode::Hex => {
                            self.view.status_view.set_body(&format!("{}-- Normal (Ascii) --{}", termion::style::Bold, termion::style::Reset)); // TODO
                            InputMode::Ascii
                        }
                        InputMode::Ascii => {
                            self.view.status_view.set_body(&format!("{}-- Normal (Hex) --{}", termion::style::Bold, termion::style::Reset)); // TODO
                            InputMode::Hex
                        }
                        //InputMode::Binary => unimplemented!(),
                    };
                    VimState::Normal
                }
                Char('a') => {
                    self.change_to_insert_mode();
                    self.update(Msg::Move(Direction::Right));
                    VimState::Insert(InputStateMachine::new(self.mode))
                }
                Char('i') => {
                    self.change_to_insert_mode();
                    VimState::Insert(InputStateMachine::new(self.mode))
                }
                Delete | Char('x') => {
                    self.yank = Some(
                        self.model.buffer[self.model.get_index()..self.model.get_index() + 1]
                            .to_owned(),
                    );
                    self.remove_right();
                    self.model.snapshot();
                    VimState::Normal
                }
                Char('r') => {
                    self.change_to_replace_mode();
                    VimState::Replace(InputStateMachine::new(self.mode), false)
                }
                Char('R') => {
                    self.change_to_replace_mode();
                    VimState::Replace(InputStateMachine::new(self.mode), true)
                }
                Char('v') => {
                    self.change_to_visual_mode();
                    VimState::Visual
                }
                Char(':') => {
                    self.change_to_command_mode();
                    VimState::Command(String::new())
                }
                Char('\n') => {
                    self.update(Msg::Move(Direction::Down));
                    self.set_index_aligned();
                    VimState::Normal
                }
                Ctrl('c') => {
                    let bytes =
                        &self.model.buffer[self.model.get_index()..self.model.get_index() + 1];
                    match save_to_clipboard(bytes) {
                        Ok(msg) | Err(msg) => self.view.status_view.set_body(&msg),
                    };

                    VimState::Normal
                }
                Char('y') => {
                    self.yank = Some(
                        self.model.buffer[self.model.get_index()..self.model.get_index() + 1]
                            .to_owned(),
                    );
                    VimState::Normal
                }
                Char('p') => {
                    if let Some(value) = self.yank.clone() {
                        let index = self.model.get_index() + 1;
                        self.paste(index, &value);
                        self.model.snapshot();
                    } else {
                        //
                    }
                    VimState::Normal
                }
                Char('P') => {
                    if let Some(value) = self.yank.clone() {
                        let index = self.model.get_index();
                        self.paste(index, &value);
                        self.update(Msg::Move(Direction::Left));
                        self.model.snapshot();
                    } else {
                        //
                    }
                    VimState::Normal
                }
                Char('u') => {
                    if !self.model.undo() {
                        self.view.status_view.set_body("Nothing to undo");
                    }
                    self.view.hex_view.scroll_to(self.model.get_index());
                    VimState::Normal
                }
                Ctrl('r') => {
                    if !self.model.redo() {
                        self.view.status_view.set_body("Nothing to redo");
                    }
                    self.view.hex_view.scroll_to(self.model.get_index());
                    VimState::Normal
                }
                Esc => {
                    self.change_to_normal_mode();
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
                            self.remove_left();
                            self.model.snapshot();
                            VimState::Insert(machine)
                        }
                        Delete => {
                            self.remove_right();
                            self.model.snapshot();
                            VimState::Insert(machine)
                        }
                        Insert => {
                            self.change_to_replace_mode();
                            VimState::Replace(InputStateMachine::new(self.mode), true)
                        }
                        Char(a) if machine.valid_input(a) => {
                            machine.transition(key);
                            match machine.state.clone() {
                                InputState::Done(byte) => {
                                    self.insert(byte);
                                    self.model.snapshot();
                                    VimState::Insert(InputStateMachine::new(self.mode))
                                }
                                InputState::Incomplete(_) => VimState::Insert(machine),
                            }
                        }
                        Char('\t') => {
                            self.mode = match self.mode {
                                InputMode::Hex => {
                                    self.view.status_view.set_body(&format!("{}-- Insert (Ascii) --{}", termion::style::Bold, termion::style::Reset)); // TODO
                                    InputMode::Ascii
                                }
                                InputMode::Ascii => {
                                    self.view.status_view.set_body(&format!("{}-- Insert (Hex) --{}", termion::style::Bold, termion::style::Reset)); // TODO
                                    InputMode::Hex
                                }
                                //InputMode::Binary => unimplemented!(),
                            };
                            VimState::Insert(InputStateMachine::new(self.mode))
                        }
                        Ctrl('v') => {
                            match read_from_clipboard() {
                                Ok(value) => {
                                    let index = self.model.get_index();
                                    self.paste(index, &value);
                                    self.model.snapshot();
                                }
                                Err(ref e) => {
                                    self.view.status_view.set_body(e);
                                }
                            }

                            VimState::Insert(machine)
                        }
                        Esc => {
                            self.change_to_normal_mode();
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
                                    self.insert(byte);
                                    self.model.snapshot();
                                    VimState::Insert(InputStateMachine::new(self.mode))
                                }
                                InputState::Incomplete(_) => VimState::Insert(machine),
                            }
                        }
                        Esc => {
                            self.change_to_normal_mode();
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
                                    self.replace(byte);
                                    self.model.snapshot();
                                    if many {
                                        self.update(Msg::Move(Direction::Right));
                                        VimState::Replace(InputStateMachine::new(self.mode), many)
                                    } else {
                                        self.change_to_normal_mode();
                                        VimState::Normal
                                    }
                                }
                                InputState::Incomplete(_) => VimState::Replace(machine, many),
                            }
                        }
                        Char('\t') => {
                            self.mode = match self.mode {
                                InputMode::Hex => {
                                    self.view.status_view.set_body(&format!("{}-- Replace (Ascii) --{}", termion::style::Bold, termion::style::Reset)); // TODO
                                    InputMode::Ascii
                                }
                                InputMode::Ascii => {
                                    self.view.status_view.set_body(&format!("{}-- Replace (Hex) --{}", termion::style::Bold, termion::style::Reset)); // TODO
                                    InputMode::Hex
                                }
                                //InputMode::Binary => unimplemented!(),
                            };
                            VimState::Replace(InputStateMachine::new(self.mode), many)
                        }
                        Esc => {
                            self.change_to_normal_mode();
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
                                    self.replace(byte);
                                    self.model.snapshot();
                                    if many {
                                        self.update(Msg::Move(Direction::Right));
                                        VimState::Replace(InputStateMachine::new(self.mode), many)
                                    } else {
                                        self.change_to_normal_mode();
                                        VimState::Normal
                                    }
                                }
                                InputState::Incomplete(_) => VimState::Replace(machine, many),
                            }
                        }
                        Esc => {
                            self.change_to_normal_mode();
                            VimState::Normal
                        }
                        _ => VimState::Replace(machine, many),
                    }
                }
            }
            VimState::Visual => match key {
                Left | Right | Up | Down | Char('h') | Char('l') | Char('k') | Char('j') => {
                    self.update(Msg::Move(Direction::try_from(key).unwrap()));
                    self.view.hex_view.scroll_to(self.model.get_index());
                    VimState::Visual
                }
                Char('y') => {
                    if let Caret::Visual(start, end) = self.model.caret {
                        let (start, end) = if usize::from(start) > usize::from(end) {
                            (end, start)
                        } else {
                            (start, end)
                        };

                        self.yank =
                            Some(self.model.buffer[start.into()..usize::from(end) + 1].to_owned());
                    } else {
                        unreachable!();
                    }
                    self.change_to_normal_mode();
                    VimState::Normal
                }
                Ctrl('c') => {
                    if let Caret::Visual(start, end) = self.model.caret {
                        let (start, end) = if usize::from(start) > usize::from(end) {
                            (end, start)
                        } else {
                            (start, end)
                        };

                        let bytes = &self.model.buffer[start.into()..usize::from(end) + 1];
                        match save_to_clipboard(bytes) {
                            Ok(msg) | Err(msg) => self.view.status_view.set_body(&msg),
                        };
                    } else {
                        unreachable!();
                    }

                    VimState::Visual
                }
                Char('o') => {
                    if let Caret::Visual(ref mut start, ref mut end) = self.model.caret {
                        swap(start, end);
                    } else {
                        panic!("wrong caret in visual state");
                    }
                    VimState::Visual
                }
                Char('x') | Char('d') => {
                    if let Caret::Visual(start, end) = self.model.caret {
                        let (start, end) = if usize::from(start) > usize::from(end) {
                            (end, start)
                        } else {
                            (start, end)
                        };

                        self.yank =
                            Some(self.model.buffer[start.into()..usize::from(end) + 1].to_owned());

                        if let Err(e) = self.model.edit(start.into(), usize::from(end) + 1, &[]) {
                            self.view
                                .status_view
                                .set_body(&format!("could not remove range ({})", e));
                        } else {
                            self.model.set_index(start.into());
                        }

                        self.view.hex_view.scroll_to(self.model.get_index());

                        self.model.snapshot();
                    } else {
                        unreachable!();
                    }
                    self.change_to_normal_mode();
                    VimState::Normal
                }
                Esc => {
                    self.change_to_normal_mode();
                    VimState::Normal
                }
                _ => VimState::Visual,
            },
            VimState::Command(mut cmd) => match key {
                Char('\n') => {
                    match Msg::parse(&cmd) {
                        Ok(cmd) => run = self.update(cmd),
                        Err(msg) => self.view.status_view.set_body(msg),
                    }
                    VimState::Normal
                }
                Backspace => {
                    cmd.pop();
                    self.view.status_view.set_body(&format!(":{}", &cmd));
                    VimState::Command(cmd)
                }
                Char(c) => {
                    cmd.push(c);
                    self.view.status_view.set_body(&format!(":{}", &cmd));
                    VimState::Command(cmd)
                }
                Esc => {
                    self.view.status_view.set_body("");
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

    impl Arbitrary for Msg {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match g.next_u32() % 2 {
                0 => Msg::Move(Direction::arbitrary(g)),
                1 => Msg::QuitWithoutSaving,
                _ => panic!(),
            }
        }
    }

    impl Arbitrary for Direction {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match g.next_u32() % 5 {
                0 => Direction::Left,
                1 => Direction::Right,
                2 => Direction::Up,
                3 => Direction::Down,
                4 => Direction::Offset(usize::arbitrary(g)),
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
