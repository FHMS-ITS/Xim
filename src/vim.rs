use crate::controller::{Direction, Msg};

use termion::event::Key::{self, Backspace, Char};

#[derive(Copy, Clone, Debug)]
pub enum InputMode {
    //Binary,
    Hex,
    Ascii,
}

#[derive(Clone, Debug)]
pub enum InputState {
    Done(u8),
    Incomplete(String),
}

#[derive(Clone, Debug)]
pub struct InputStateMachine {
    mode: InputMode,
    pub state: InputState,
}

impl InputStateMachine {
    pub fn new(mode: InputMode) -> InputStateMachine {
        InputStateMachine {
            mode: mode,
            state: InputState::Incomplete(String::new()),
        }
    }

    pub fn valid_input(&self, c: char) -> bool {
        match self.mode {
            //InputMode::Binary => is_binary(c),
            InputMode::Hex => c.is_ascii_hexdigit(),
            InputMode::Ascii => c.is_ascii_graphic(),
        }
    }

    pub fn initial(&self) -> bool {
        match self.state.clone() {
            InputState::Incomplete(vec) => vec.is_empty(),
            InputState::Done(_) => true,
        }
    }

    pub fn transition(&mut self, key: Key) {
        self.state = match self.state.clone() {
            InputState::Incomplete(mut vec) => {
                match key {
                    Backspace => {
                        vec.pop();
                        InputState::Incomplete(vec)
                    }
                    Char(x) if self.valid_input(x) => {
                        vec.push(x);
                        match self.mode {
                            /*InputMode::Binary => {
                                if vec.len() == 8 {
                                    // Safe-from-panic: This will never panic, because invalid characters can't be inserted
                                    InputState::Done(u8::from_str_radix(&vec, 2).unwrap())
                                } else {
                                    InputState::Incomplete(vec)
                                }
                            }*/
                            InputMode::Hex => {
                                if vec.len() == 2 {
                                    // Safe-from-panic: This will never panic, because invalid characters can't be inserted
                                    InputState::Done(u8::from_str_radix(&vec, 16).unwrap())
                                } else {
                                    InputState::Incomplete(vec)
                                }
                            }
                            InputMode::Ascii => {
                                if vec.len() == 1 {
                                    // Safe-from-panic: We push prior to the next() call, thus there is always at least one character
                                    InputState::Done(vec.chars().next().unwrap() as u8)
                                } else {
                                    InputState::Incomplete(vec)
                                }
                            }
                        }
                    }
                    _ => InputState::Incomplete(vec),
                }
            }
            InputState::Done(byte) => InputState::Done(byte),
        }
    }
}

#[derive(Clone)]
pub enum VimState {
    Normal,
    Insert(InputStateMachine),
    Replace(InputStateMachine, bool),
    Visual,
    Command(String),
}

impl Msg {
    pub fn parse(cmd: &str) -> Result<Msg, &'static str> {
        use self::Msg::*;

        if cmd.starts_with("w ") {
            return Ok(SaveAs(cmd[2..].trim().into()));
        }

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
                    Ok(Move(Direction::Offset(offset)))
                } else {
                    Err("no such command")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn test_msg_parse_doesnt_panic(input: String) -> bool {
        let _ = Msg::parse(&input);
        true
    }
}

