extern crate chan;
extern crate chan_signal;
extern crate clipboard;
extern crate hex;
extern crate termion;

use chan_signal::{Signal, notify};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{RawTerminal, IntoRawMode};
use termion::screen::AlternateScreen;
use std::cell::RefCell;
use std::error::Error;
use std::io::{Write, Stdout, stdout, stdin};
use std::sync::mpsc::sync_channel;
use std::thread;
use std::rc::Rc;
use std::cmp::min;
use std::ops::{Add, AddAssign, Drop, Sub, SubAssign, Rem, RemAssign};

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

mod controller;
pub mod model;
mod view;
mod history;
mod vim;

use controller::Controller;
use view::View;
use model::Model;

pub type RawStdout = Rc<RefCell<AlternateScreen<RawTerminal<Stdout>>>>;

enum Event {
    Key(Key),
    Resize((u16, u16)),
    Kill,
}

pub struct Config {
    pub file: String,
}

pub struct App {
    config: Config,
    stdout: RawStdout,
}

impl App {
    pub fn new(config: Config) -> App {
        App {
            config: config,
            stdout: Rc::new(
                RefCell::new(
                    AlternateScreen::from(stdout().into_raw_mode().unwrap())
                )
            ),
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
                        Signal::WINCH => send_1.send(Event::Resize(termion::terminal_size().unwrap())).unwrap(),
                        Signal::TERM => send_1.send(Event::Kill).unwrap(),
                        _ => {},
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

        let mut ctrl = Controller::new(
            Model::new(),
            View::new(self.stdout.clone())
        );

        ctrl.resize_view(termion::terminal_size()?);
        ctrl.open(&self.config.file);
        ctrl.update_view();

        for event in events.iter() {
            match event {
                Event::Key(k) => {
                    if !ctrl.transition(k) {
                        break
                    }
                }
                Event::Resize(new_size) => ctrl.resize_view(new_size),
                Event::Kill => break,
            }

            ctrl.update_view();
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

pub fn move_window(start: usize, height: usize, new_index: usize) -> Option<usize> {
    if height == 0 {
        return None;
    }

    let mut new_start = start;

    if new_index < start {
        new_start = new_index;
    } else if new_index > start + (height.saturating_sub(1)) {
        new_start = new_index - (height.saturating_sub(1));
    }

    Some(new_start)
}

pub trait Ascii {
    fn to_printable(self: Self) -> char;
}

impl Ascii for u8 {
    fn to_printable(self: u8) -> char {
        if self >= 32 && self <= 126  {
            self as char
        } else {
            '.'
        }
    }
}

pub trait Hex {
    fn is_hex(self: Self) -> bool;
    fn to_hex(self: Self) -> Option<u8>;
}

impl Hex for char {
    fn is_hex(self: char) -> bool {
        match self {
            '0'...'9' | 'a'...'f' | 'A'...'F' => true,
            _ => false
        }
    }

    fn to_hex(self: char) -> Option<u8> {
        match self {
            '0'...'9' => Some(self as u8 - b'0'),
            'a'...'f' => Some(self as u8 - b'a' + 10),
            'A'...'F' => Some(self as u8 - b'A' + 10),
            _ => None
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

pub fn align(value: u16, boundary: u16) -> u16 {
    if boundary == 0 {
        value
    } else {
        value - (value % boundary)
    }
}

pub fn align_top(value: u16, boundary: u16) -> u16 {
    if boundary == 0 {
        value
    } else {
        align(value, boundary) + (boundary - 1)
    }
}

#[derive(Clone, Debug)]
pub enum Caret {
    Index(UsizeMax),
    Offset(UsizeMax),
    Replace(UsizeMax),
    Visual(UsizeMax, UsizeMax),
}

#[cfg(test)]
mod tests {
    use super::*;

    quickcheck!{
        fn test_align(index: u16, random: u16, boundary: u16) -> bool {
            match boundary {
                0 => align(index, boundary) == index,
                1 => align(index, boundary) == index,
                _ => align(index * boundary + (random % boundary), boundary) == index * boundary,
            }
        }
    }

    quickcheck!{
        fn test_align_top(index: u16, random: u16, boundary: u16) -> bool {
            match boundary {
                0 => align_top(index, boundary) == index,
                1 => align_top(index, boundary) == index,
                _ => align_top(index * boundary + (random % boundary), boundary) == index * boundary + (boundary - 1),
            }
        }
    }

    quickcheck!{
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

    quickcheck!{
        fn test_move_window(start: usize, height: usize, index: usize) -> bool {
            if let Some(new_start) = move_window(start, height, index) {
                // Do not move when unnecessary...
                if start <= index && index <= start + (height - 1) {
                    new_start == start
                } else {
                    // ...and always be in range...
                    new_start <= index && index <= new_start + height
                }
            } else {
                height == 0
            }
        }
    }
}