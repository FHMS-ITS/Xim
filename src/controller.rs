use super::view::*;
use super::model::*;
use super::{Caret, char_is_hex, char_to_hex};
use super::vim::{VimState, VimCommand};

use termion;
use termion::event::Key;
use std::mem::swap;

pub struct Controller {
    pub state: VimState,
    pub model: Model,
    pub view: View,
}

impl Controller {
    pub fn new(model: Model, view: View) -> Controller {
        Controller {
            state: VimState::Normal,
            model: model,
            view: view,
        }
    }

    // Opening, Saving, etc.

    pub fn open(&mut self, path: &str) {
        match self.model.open(path) {
            Ok(_) => {
                self.view.status_view.set_head(path)
            }
            Err(e) => {
                self.view.status_view.set_head(&format!("error: {}", e))
            }
        }
    }

    pub fn save(&mut self) -> bool {
        match self.model.save() {
            Ok(_) => {
                self.view.status_view.set_body(&format!("\"{}\" saved", self.model.path));
                true
            },
            Err(error) => {
                self.view.status_view.set_body(&format!("could not save \"{}\": {}", self.model.path, error));
                false
            }
        }
    }

    // Moving

    pub fn into_normal_mode(&mut self) {
        self.model.into_offset_mode();
        self.view.status_view.set_body(&format!("{}-- Normal --{}", termion::style::Bold, termion::style::Reset));
    }

    pub fn into_insert_mode(&mut self) {
        self.model.into_insert_mode();
        self.view.status_view.set_body(&format!("{}-- Insert --{}", termion::style::Bold, termion::style::Reset));
    }

    pub fn into_replace_mode(&mut self) {
        self.model.into_replace_mode();
        self.view.status_view.set_body(&format!("{}-- Replace --{}", termion::style::Bold, termion::style::Reset));
    }

    pub fn into_command_mode(&mut self) {
        self.view.status_view.set_body(":")
    }

    pub fn into_visual_mode(&mut self) {
        self.model.into_visual_mode();
        self.view.status_view.set_body(&format!("{}-- Visual --{}", termion::style::Bold, termion::style::Reset));
    }

    pub fn make_move(&mut self, direction: termion::event::Key) {
        use termion::event::Key::{Left, Right, Up, Down, Char};

        match direction {
            Left | Char('h') => self.model.dec_index(1),
            Right | Char('l') => self.model.inc_index(1),
            Up | Char('k') => self.model.dec_index(16),
            Down | Char('j') => self.model.inc_index(16),
            _ => panic!("make_move called with non-move key"),
        };

        self.view.hex_view.scroll_to(self.model.get_index());
        self.view.status_view.set_index(self.model.get_index());
    }

    // Index

    pub fn set_index(&mut self, offset: usize) {
        self.model.set_index(offset);
        self.view.hex_view.scroll_to(self.model.get_index());

        let index = match self.model.caret {
            Caret::Index(index) => index,
            Caret::Offset(index) => index,
            Caret::Replace(index) => index,
            Caret::Visual(_, end) => end,
        };

        self.view.status_view.set_index(index.into());
    }

    pub fn set_index_aligned(&mut self) {
        self.model.set_index_aligned();
    }

    pub fn get_index(&self) -> usize {
        match self.model.caret {
            Caret::Index(index) |
            Caret::Replace(index) |
            Caret::Offset(index) |
            Caret::Visual(_, index) => index.into(),
        }
    }

    // Editing

    pub fn insert(&mut self, value: u8) {
        let index = self.model.get_index();
        if let Err(e) = self.model.edit(index, index, &[value]) {
            self.view.status_view.set_body(&format!("could not insert value ({})", e));
        }
        self.model.inc_index(1);
    }

    pub fn remove_left(&mut self) {
        let index = self.model.get_index();

        // If cursor is at end, deleting will move the cursor to the left since the range updates automatically...
        let end = index == self.model.buffer.len();

        if let Err(e) = self.model.edit(index.saturating_sub(1), index, &[]) {
            self.view.status_view.set_body(&format!("could not remove value ({})", e));
        }

        // ...thus do not move again. (TODO: Refactor)
        if !end {
            self.model.dec_index(1);
        }
    }

    pub fn remove_right(&mut self) {
        let index = self.model.get_index();
        if let Err(e) = self.model.edit(index,index.saturating_add(1), &[]) {
            self.view.status_view.set_body(&format!("could not remove value ({})", e));
        }
    }

    pub fn replace(&mut self, value: u8) {
        let index = self.model.get_index();
        if let Err(e) = self.model.edit(index, index.saturating_add(1), &[value]) {
            self.view.status_view.set_body(&format!("could not replace value ({})", e));
        }
    }

    // Views

    pub fn resize_view(&mut self, size: (u16, u16)) {
        self.view.set_area(DrawArea {
            origin: (1, 1),
            dimens: size
        });
    }

    pub fn update_view(&mut self) {
        self.view.draw(&self.model);
    }

    // Transitions

    // TODO: Refactor into VimStateMachine
    pub fn transition(&mut self, key: Key) -> bool {
        use self::VimState::*;
        use termion::event::Key::{Char, Esc, Delete, Backspace, Left, Right, Up, Down, Insert, Ctrl};

        let mut run = true;

        // TODO: Pull state machine
        /*
        self.state = match self.state.clone() {
            Normal => {
                match next_key() {
                    Char('y') => {
                        match next_key() {
                            Char('y') => {},
                            _ => {},
                        }
                    },
                    _ => {},
                }
            },
            _ => {},
        };
        */

        self.state = match self.state.clone() {
            Normal => {
                match key {
                    Left | Right | Up | Down | Char('h') | Char('l') | Char('k') | Char('j') => {
                        self.make_move(key);
                        Normal
                    }
                    Backspace => {
                        self.make_move(Left);
                        Normal
                    }
                    Char('\t') => {
                        self.view.status_view.set_body("Changing to ASCII not implemented yet");
                        Normal
                    }
                    Char('i') => {
                        self.into_insert_mode();
                        Insert1
                    },
                    Delete | Char('x') => {
                        self.remove_right();
                        self.model.snapshot();
                        Normal
                    }
                    Char('r') => {
                        self.into_replace_mode();
                        Replace1
                    },
                    Char('R') => {
                        self.into_replace_mode();
                        ReplaceMany1
                    },
                    Char('v') => {
                        self.into_visual_mode();
                        Visual
                    }
                    Char(':') => {
                        self.into_command_mode();
                        Command(String::new())
                    },
                    Char('\n') => {
                        self.make_move(Down);
                        self.set_index_aligned();
                        Normal
                    }
                    Char('u') => {
                        if !self.model.undo() {
                            self.view.status_view.set_body("Nothing to undo");
                        }
                        Normal
                    }
                    Ctrl('r') => {
                        if !self.model.redo() {
                            self.view.status_view.set_body("Nothing to redo");
                        }
                        Normal
                    }
                    Esc => {
                        self.into_normal_mode();
                        Normal
                    }
                    _ => Normal,
                }
            }
            Insert1 => {
                match key {
                    Left | Right | Up | Down => {
                        self.make_move(key);
                        Insert1
                    }
                    Backspace => {
                        self.remove_left();
                        self.model.snapshot();
                        Insert1
                    }
                    Delete => {
                        self.remove_right();
                        self.model.snapshot();
                        Insert1
                    }
                    Char(a) if char_is_hex(a) => {
                        Insert2(char_to_hex(a).unwrap())
                    },
                    Insert => {
                        self.into_replace_mode();
                        ReplaceMany1
                    }
                    Esc => {
                        self.into_normal_mode();
                        Normal
                    },
                    _ => Insert1
                }
            }
            Insert2(a) => {
                match key {
                    Char(b) if char_is_hex(b) => {
                        self.insert(16*a + char_to_hex(b).unwrap());
                        self.model.snapshot();
                        Insert1
                    }
                    Esc => {
                        self.into_normal_mode();
                        Normal
                    },
                    _ => Insert2(a)
                }
            }
            Replace1 => {
                match key {
                    Char(a) if char_is_hex(a) => {
                        Replace2(char_to_hex(a).unwrap())
                    },
                    Esc => {
                        self.into_normal_mode();
                        Normal
                    },
                    _ => Replace1
                }
            }
            Replace2(a) => {
                match key {
                    Char(b) if char_is_hex(b) => {
                        self.into_normal_mode();
                        self.replace(16*a + char_to_hex(b).unwrap());
                        self.model.snapshot();
                        Normal
                    }
                    Esc => {
                        self.into_normal_mode();
                        Normal
                    },
                    _ => Replace2(a)
                }
            }
            ReplaceMany1 => {
                match key {
                    Left | Right | Up | Down => {
                        self.make_move(key);
                        ReplaceMany1
                    }
                    Char(a) if char_is_hex(a) => {
                        ReplaceMany2(char_to_hex(a).unwrap())
                    },
                    Esc => {
                        self.into_normal_mode();
                        Normal
                    },
                    _ => ReplaceMany1
                }
            }
            ReplaceMany2(a) => {
                match key {
                    Char(b) if char_is_hex(b) => {
                        self.replace(16*a + char_to_hex(b).unwrap());
                        self.model.snapshot();
                        self.make_move(Right);
                        ReplaceMany1
                    }
                    Esc => {
                        self.into_normal_mode();
                        Normal
                    },
                    _ => ReplaceMany2(a)
                }
            }
            Visual => {
                match key {
                    Left | Right | Up | Down | Char('h') | Char('l') | Char('k') | Char('j') => {
                        self.make_move(key);
                        Visual
                    },
                    Char('o') => {
                        if let Caret::Visual(ref mut start, ref mut end) = self.model.caret {
                            swap(start, end);
                        } else {
                            panic!("wrong caret in visual state");
                        }
                        Visual
                    },
                    Char('x') | Char('d') => {
                        if let Caret::Visual(start, end) = self.model.caret {
                            let (start, end) = if usize::from(start) > usize::from(end) {
                                (end, start)
                            } else {
                                (start, end)
                            };

                            if let Err(e) = self.model.edit(start.into(), usize::from(end) + 1, &[]) {
                                self.view.status_view.set_body(&format!("could not remove range ({})", e));
                            } else {
                                self.model.set_index(start.into());
                            }

                            self.model.snapshot();
                        } else {
                            unreachable!();
                        }
                        self.into_normal_mode();
                        Normal
                    },
                    Esc => {
                        self.into_normal_mode();
                        Normal
                    }
                    _ => Visual
                }
            }
            Command(mut cmd) => {
                match key {
                    Char('\n') => {
                        match VimCommand::parse(&cmd) {
                            Ok(cmd) => {
                                match cmd {
                                    VimCommand::Quit => {
                                        if self.model.is_modified() {
                                            self.view.status_view.set_body("save your changes with :w or force quit with :q!");
                                        } else {
                                            run = false;
                                        }
                                    },
                                    VimCommand::QuitWithoutSaving => {
                                        run = false;
                                    }
                                    VimCommand::Save => {
                                        self.save();
                                    },
                                    VimCommand::SaveAndQuit => {
                                        if self.save() {
                                            run = false;
                                        }
                                    },
                                    VimCommand::Jump(offset) => {
                                        self.set_index(offset);
                                        self.view.status_view.set_body(&"");
                                    },
                                }
                            },
                            Err(msg) => {
                                self.view.status_view.set_body(msg);
                            },
                        }
                        Normal
                    }
                    Backspace => {
                        cmd.pop();
                        self.view.status_view.set_body(&format!(":{}", &cmd));
                        Command(cmd)
                    }
                    Char(c) => {
                        cmd.push(c);
                        self.view.status_view.set_body(&format!(":{}", &cmd));
                        Command(cmd)
                    }
                    Esc => {
                        self.view.status_view.set_body(&"");
                        Normal
                    },
                    _ => Command(cmd)
                }
            }
        };

        run
    }
}