extern crate termion;

pub mod model;
pub mod view;
pub mod controller;

mod vim;
mod history;

use std::cmp::min;
use std::ops::{Add, AddAssign, Sub, SubAssign, Rem, RemAssign};

#[derive(Clone, Copy)]
pub struct UsizeMax {
    value: usize,
    max: usize,
}

impl UsizeMax {
    pub fn new(value: usize, max: usize) -> UsizeMax {
        let mut ret = UsizeMax { value, max };
        ret.clamp();
        ret
    }

    pub fn set_value(&mut self, new_value: usize) {
        self.value = new_value;
        self.clamp();
    }

    pub fn set_maximum(&mut self, max: usize) {
        self.max = max;
        self.clamp();
    }

    pub fn get_maximum(&self) -> usize {
        self.max
    }

    fn clamp(&mut self) {
        self.value = min(self.value, self.max);
    }
}

impl Add<usize> for UsizeMax {
    type Output = UsizeMax;

    fn add(mut self, other: usize) -> UsizeMax {
        self.value = self.value.saturating_add(other);
        self.clamp();
        self
    }
}

impl AddAssign<usize> for UsizeMax {
    fn add_assign(&mut self, other: usize) {
        self.value = self.value.saturating_add(other);
        self.clamp();
    }
}

impl<'a> AddAssign<usize> for &'a mut UsizeMax {
    fn add_assign(&mut self, other: usize) {
        self.value = self.value.saturating_add(other);
        self.clamp();
    }
}

impl Sub<usize> for UsizeMax {
    type Output = UsizeMax;

    fn sub(mut self, other: usize) -> UsizeMax {
        self.value = self.value.saturating_sub(other);
        self.clamp();
        self
    }
}

impl SubAssign<usize> for UsizeMax {
    fn sub_assign(&mut self, other: usize) {
        self.value = self.value.saturating_sub(other);
        self.clamp();
    }
}

impl<'a> SubAssign<usize> for &'a mut UsizeMax {
    fn sub_assign(&mut self, other: usize) {
        self.value = self.value.saturating_sub(other);
        self.clamp();
    }
}

impl Rem<usize> for UsizeMax {
    type Output = UsizeMax;

    fn rem(mut self, other: usize) -> UsizeMax {
        self.value = self.value % other;
        self.clamp();
        self
    }
}

impl RemAssign<usize> for UsizeMax {
    fn rem_assign(&mut self, other: usize) {
        self.value = self.value % other;
        self.clamp();
    }
}

impl<'a> RemAssign<usize> for &'a mut UsizeMax {
    fn rem_assign(&mut self, other: usize) {
        self.value = self.value % other;
        self.clamp();
    }
}

impl From<UsizeMax> for usize {
    fn from(mut convertee: UsizeMax) -> Self {
        convertee.clamp();
        convertee.value
    }
}

impl<'a> From<&'a mut UsizeMax> for usize {
    fn from(convertee: &'a mut UsizeMax) -> Self {
        convertee.clamp();
        convertee.value
    }
}

pub fn align(value: u16, boundary: u16) -> u16 {
    return if boundary == 0 {
        value
    } else {
        value - (value % boundary)
    }
}

pub fn align_top(value: u16, boundary: u16) -> u16 {
    return if boundary == 0 {
        value
    } else {
        align(value, boundary) + (boundary - 1)
    }
}

pub fn chunks_indices(mut start: u16, end: u16, size: u16) -> Vec<(u16, u16)> {
    use std::cmp::min;

    let mut result = Vec::with_capacity(((end - start) / 16) as usize);

    while start <= end {
        result.push((start, min(start + size - 1, end)));
        start += size;
    }

    result
}

pub fn range_to_marker(start: u16, end: u16) -> Vec<(u16, u16, u16)> {
    let (start, end) = if start < end {
        (start, end)
    } else {
        (end, start)
    };

    let lines = (start/16..end/16 + 1).collect::<Vec<_>>();
    let mut spans = chunks_indices(align(start, 16), align_top(end, 16), 16);
    spans.first_mut().unwrap().0 += start;
    spans.last_mut().unwrap().1 = end % 16;

    lines.iter().zip(spans.iter()).map(|(line, &(x, y))| (*line, x % 16, y % 16)).collect()
}

pub fn char_to_ascii_printable(byte: u8) -> char {
    if byte >= 32 && byte <= 126  {
        byte as char
    } else {
        '.'
    }
}

fn char_is_hex(c: char) -> bool {
    match c {
        '0'...'9' | 'a'...'f' | 'A'...'F' => true,
        _ => false
    }
}

fn char_to_hex(c: char) -> Option<u8> {
    match c {
        '0'...'9' => Some(c as u8 - '0' as u8),
        'a'...'f' => Some(c as u8 - 'a' as u8 + 10),
        'A'...'F' => Some(c as u8 - 'A' as u8 + 10),
        _ => None
    }
}

pub fn clamp<T: Ord>(num: T, (min, max): (T, T)) -> Option<T> {
    if min <= max {
        Some(
            if num < min {
                min
            } else if num > max {
                max
            } else {
                num
            }
        )
    } else {
        None
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

    /*
    #[test]
    fn test_clamp() {
        assert_eq!(clamp(0, (0, 0)), Some(0));
        assert_eq!(clamp(1, (0, 1)), Some(1));
        assert_eq!(clamp(1, (0, 1)), Some(1));
        assert_eq!(clamp(2, (0, 1)), Some(1));
        assert_eq!(clamp(2, (0, 1)), Some(1));

        assert_eq!(clamp(50, (50, 100)), Some(50));
        assert_eq!(clamp(75, (50, 100)), Some(75));
        assert_eq!(clamp(100, (50, 100)), Some(100));
        assert_eq!(clamp(125, (50, 100)), Some(100));

        assert_eq!(clamp(550, (500, 100)), None);
        assert_eq!(clamp(100, (500, 100)), None);
    }
    */

    quickcheck!{
        fn test_clamp_quick(value: usize, min: usize, max: usize) -> bool {
            if min <= max {
                if let Some(result) = clamp(value, (min, max)) {
                    result >= min && result <= max
                } else {
                    false
                }
            } else {
                clamp(value, (min, max)) == None
            }
        }
    }

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