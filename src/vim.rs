use termion::event::Key::{self, Char, Esc, Backspace};

fn is_binary(c: char) -> bool {
    match c {
        '0'...'1' => true,
        _ => false
    }
}

fn is_hex(c: char) -> bool {
    match c {
        '0'...'9' |
        'a'...'f' |
        'A'...'F' => true,
        _ => false
    }
}

// TODO: use std::ascii::AsciiExt when stable
fn is_printable(c: char) -> bool {
    c as u8 >= 32 && c as u8 <= 126
}

#[derive(Copy, Clone, Debug)]
pub enum InsertMode {
    Binary,
    Hex,
    Ascii,
}

#[derive(Clone, Debug)]
pub enum InsertState {
    Done(u8),
    Error,
    Incomplete(String),
}

#[derive(Clone, Debug)]
pub struct ValueStateMachine {
    mode: InsertMode,
    pub state: InsertState,
}

use self::InsertMode::*;
use self::InsertState::*;

impl ValueStateMachine {
    pub fn new(mode: InsertMode) -> ValueStateMachine {
        ValueStateMachine {
            mode: mode,
            state: Incomplete(String::new()),
        }
    }

    pub fn valid_char(&self, c: char) -> bool {
        match self.mode {
            Binary => is_binary(c),
            Hex => is_hex(c),
            Ascii => is_printable(c),
        }
    }

    pub fn transition(&mut self, key: Key) {
        self.state = match self.state.clone() {
            Incomplete(mut vec) => {
                match key {
                    Backspace => {
                        vec.pop();
                        Incomplete(vec)
                    }
                    Char(x) => {
                        vec.push(x);
                        match self.mode {
                            InsertMode::Binary => {
                                if vec.len() == 8 {
                                    if let Ok(byte) = u8::from_str_radix(&vec, 2) {
                                        Done(byte)
                                    } else {
                                        Error
                                    }
                                } else {
                                    Incomplete(vec)
                                }
                            }
                            InsertMode::Hex => {
                                if vec.len() == 2 {
                                    if let Ok(byte) = u8::from_str_radix(&vec, 16) {
                                        Done(byte)
                                    } else {
                                        Error
                                    }
                                } else {
                                    Incomplete(vec)
                                }
                            }
                            InsertMode::Ascii => {
                                if vec.len() == 1 {
                                    if let Some(c) = vec.chars().next() {
                                        Done(c as u8)
                                    } else {
                                        Error
                                    }
                                } else {
                                    Incomplete(vec)
                                }
                            }
                        }
                    }
                    _ => Error
                }
            }
            _ => Error,
        }
    }
}

// TODO: Merge with planned refactoring with Command-Pattern?
pub enum VimCommand {
    Quit,
    QuitWithoutSaving,
    Save,
    SaveAndQuit,
    Jump(usize),
}

impl VimCommand {
    pub fn parse(cmd: &str) -> Result<VimCommand, &'static str> {
        use self::VimCommand::*;

        match cmd {
            "q" => Ok(Quit),
            "q!" => Ok(QuitWithoutSaving),
            "w" => Ok(Save),
            "wq" | "x" => Ok(SaveAndQuit),
            offset => {
                // If none of the above commands, try to interpret as jump command...

                let (skip, base) = if offset.starts_with("0b") {
                    (2, 2)
                } else if offset.starts_with("08") {
                    (2, 8)
                } else if offset.starts_with("0x") {
                    (2, 16)
                } else {
                    (0, 10)
                };

                // ...and error out if no valid offset. (Proper parsing may be implemented in the future.)
                if let Ok(offset) = usize::from_str_radix(&offset[skip..], base) {
                    Ok(Jump(offset))
                } else {
                    Err("no such command")
                }
            },
        }
    }
}

#[derive(Clone)]
pub enum VimState {
    Normal,
    Insert(ValueStateMachine),
    Replace(ValueStateMachine, bool),
    Visual,
    Command(String),
}