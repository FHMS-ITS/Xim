use std::{
    cell::RefCell,
    cmp::min,
    error::Error,
    io::{stdin, stdout, Stdout, Write},
    ops::{Add, AddAssign, Drop, Rem, RemAssign, Sub, SubAssign},
    rc::Rc,
    sync::mpsc::sync_channel,
    thread,
};

use chan_signal::{notify, Signal};
use termion::{
    event::Key,
    input::TermRead,
    raw::{IntoRawMode, RawTerminal},
    screen::AlternateScreen,
};

mod controller;
mod history;
mod model;
mod utils;
mod view;
mod vim;

use {
    controller::{Controller, Msg},
    model::Model,
    view::View,
};

pub type RawStdout = Rc<RefCell<AlternateScreen<RawTerminal<Stdout>>>>;

enum Event {
    Key(Key),
    Resize((u16, u16)),
    Kill,
}

pub struct Args {
    pub file: String,
}

pub struct App {
    args: Args,
    stdout: RawStdout,
}

impl App {
    pub fn new(args: Args) -> App {
        App {
            args: args,
            stdout: Rc::new(RefCell::new(AlternateScreen::from(
                stdout().into_raw_mode().unwrap(),
            ))),
        }
    }

    pub fn run(mut self) -> Result<(), Box<Error>> {
        self.setup_terminal()?;

        let events = {
            // Create event channel
            let (send, recv) = sync_channel(0);

            // Listen for window changed and terminate signals
            let signals = notify(&[Signal::WINCH, Signal::TERM]);

            let send_1 = send.clone();
            thread::spawn(move || {
                for signal in signals.iter() {
                    match signal {
                        Signal::WINCH => send_1
                            .send(Event::Resize(termion::terminal_size().unwrap()))
                            .unwrap(),
                        Signal::TERM => send_1.send(Event::Kill).unwrap(),
                        _ => {}
                    }
                }
            });

            // Receive keypress events
            let send_2 = send.clone();
            thread::spawn(move || {
                for c in stdin().keys() {
                    send_2.send(Event::Key(c.unwrap())).unwrap();
                }
            });

            recv
        };

        let mut ctrl = Controller::new(Model::new(), View::new(self.stdout.clone()));

        ctrl.update(Msg::Resize(termion::terminal_size()?));
        ctrl.update(Msg::Open(self.args.file.clone()));
        ctrl.update(Msg::Redraw);

        for event in events.iter() {
            match event {
                Event::Key(k) => {
                    if !ctrl.transition(k) {
                        break;
                    }
                }
                Event::Resize(new_size) => {
                    ctrl.update(Msg::Resize(new_size));
                }
                Event::Kill => break,
            }

            ctrl.update(Msg::Redraw);
        }

        Ok(())
    }

    fn setup_terminal(&mut self) -> Result<(), Box<Error>> {
        let mut stdout = self.stdout.borrow_mut();
        write!(stdout, "{}", termion::cursor::Hide)?;
        write!(stdout, "{}", termion::clear::All)?;
        stdout.flush()?;
        Ok(())
    }

    fn teardown_terminal(&mut self) -> Result<(), Box<Error>> {
        let mut stdout = self.stdout.borrow_mut();
        write!(stdout, "{}", termion::clear::All)?;
        write!(stdout, "{}", termion::cursor::Show)?;
        stdout.flush()?;
        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Err(error) = self.teardown_terminal() {
            eprintln!("{}", error);
        }
    }
}

pub trait Ascii {
    fn to_printable(self: Self) -> char;
}

impl Ascii for u8 {
    fn to_printable(self: u8) -> char {
        if self >= 32 && self <= 126 {
            self as char
        } else {
            '.'
        }
    }
}

// TODO: worth the effort?
impl UsizeMax {
    pub fn new(value: usize, max: usize) -> UsizeMax {
        let mut ret = UsizeMax { value, max };
        ret.adjust();
        ret
    }

    pub fn set_value(&mut self, new_value: usize) {
        self.value = new_value;
        self.adjust();
    }

    pub fn set_maximum(&mut self, max: usize) {
        self.max = max;
        self.adjust();
    }

    pub fn get_maximum(&self) -> usize {
        self.max
    }

    fn adjust(&mut self) {
        self.value = min(self.value, self.max);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UsizeMax {
    value: usize,
    max: usize,
}

impl Add<usize> for UsizeMax {
    type Output = UsizeMax;

    fn add(mut self, other: usize) -> UsizeMax {
        self.value = self.value.saturating_add(other);
        self.adjust();
        self
    }
}

impl AddAssign<usize> for UsizeMax {
    fn add_assign(&mut self, other: usize) {
        self.value = self.value.saturating_add(other);
        self.adjust();
    }
}

impl<'a> AddAssign<usize> for &'a mut UsizeMax {
    fn add_assign(&mut self, other: usize) {
        self.value = self.value.saturating_add(other);
        self.adjust();
    }
}

impl Sub<usize> for UsizeMax {
    type Output = UsizeMax;

    fn sub(mut self, other: usize) -> UsizeMax {
        self.value = self.value.saturating_sub(other);
        self.adjust();
        self
    }
}

impl SubAssign<usize> for UsizeMax {
    fn sub_assign(&mut self, other: usize) {
        self.value = self.value.saturating_sub(other);
        self.adjust();
    }
}

impl<'a> SubAssign<usize> for &'a mut UsizeMax {
    fn sub_assign(&mut self, other: usize) {
        self.value = self.value.saturating_sub(other);
        self.adjust();
    }
}

impl Rem<usize> for UsizeMax {
    type Output = UsizeMax;

    fn rem(mut self, other: usize) -> UsizeMax {
        self.value %= other;
        self.adjust();
        self
    }
}

impl RemAssign<usize> for UsizeMax {
    fn rem_assign(&mut self, other: usize) {
        self.value = self.value % other;
        self.adjust();
    }
}

impl<'a> RemAssign<usize> for &'a mut UsizeMax {
    fn rem_assign(&mut self, other: usize) {
        self.value = self.value % other;
        self.adjust();
    }
}

impl From<UsizeMax> for usize {
    fn from(mut convertee: UsizeMax) -> Self {
        convertee.adjust();
        convertee.value
    }
}

impl<'a> From<&'a mut UsizeMax> for usize {
    fn from(convertee: &'a mut UsizeMax) -> Self {
        convertee.adjust();
        convertee.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn test_usizemax(value: usize, max: usize, operations: Vec<(u8, usize)>) -> bool {
        let mut value = UsizeMax::new(value, max);

        for (operator, rhs) in operations {
            match operator % 2 {
                0 => value += rhs,
                1 => value -= rhs,
                _ => unreachable!(),
            }
        }

        usize::from(value) <= max
    }
}
