extern crate termion;

pub mod controller;
pub mod model;
pub mod view;

mod history;
mod vim;

use std::cmp::min;
use std::ops::{Add, AddAssign, Drop, Sub, SubAssign, Rem, RemAssign};

pub struct Config {
    pub file: String,
}

pub struct App;

impl App {
    pub fn new() -> App {
        App
    }

    pub fn with(config: Config) -> App {
        App
    }

    pub fn run() {
        unimplemented!();
    }
}

impl Drop for App {
    fn drop(&mut self) {
        println!("Exiting App");
    }
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
            '0'...'9' => Some(self as u8 - '0' as u8),
            'a'...'f' => Some(self as u8 - 'a' as u8 + 10),
            'A'...'F' => Some(self as u8 - 'A' as u8 + 10),
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

#[derive(Clone, Copy)]
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
        self.value = self.value % other;
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

#[derive(Clone)]
pub enum Caret {
    Index(UsizeMax),
    Offset(UsizeMax),
    Replace(UsizeMax),
    Visual(UsizeMax, UsizeMax),
}

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

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

    #[test]
    fn test_chunks_indices() {
        assert_eq!(chunks_indices(0, 16, 5), vec![(0, 4), (5, 9), (10, 14), (15, 16)]);
        assert_eq!(chunks_indices(0, 6, 2), vec![(0, 1), (2, 3), (4, 5), (6, 6)]);
        assert_eq!(chunks_indices(0, 3, 1), vec![(0, 0), (1, 1), (2, 2), (3, 3)]);
        assert_eq!(chunks_indices(0, 10, 3), vec![(0, 2), (3, 5), (6, 8), (9, 10)]);
        assert_eq!(chunks_indices(0, 11, 3), vec![(0, 2), (3, 5), (6, 8), (9, 11)]);
        assert_eq!(chunks_indices(10, 11, 3), vec![(10, 11)]);
        assert_eq!(chunks_indices(10, 15, 7), vec![(10, 15)]);
        assert_eq!(chunks_indices(13, 19, 6), vec![(13, 18), (19, 19)]);
    }

    #[test]
    fn test_range_to_marker() {
        assert_eq!(range_to_marker(0, 16), vec![(0, 0, 15), (1, 0, 0)]);
        assert_eq!(range_to_marker(8, 18), vec![(0, 8, 15), (1, 0, 2)]);
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
}