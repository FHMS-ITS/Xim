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

#[cfg(test)]
mod tests {
    use super::*;

    use quickcheck::quickcheck;

    quickcheck! {
        fn test_align(index: u16, random: u16, boundary: u16) -> bool {
            match boundary {
                0 => align(index, boundary) == index,
                1 => align(index, boundary) == index,
                _ => align(index * boundary + (random % boundary), boundary) == index * boundary,
            }
        }
    }

    quickcheck! {
        fn test_align_top(index: u16, random: u16, boundary: u16) -> bool {
            match boundary {
                0 => align_top(index, boundary) == index,
                1 => align_top(index, boundary) == index,
                _ => align_top(index * boundary + (random % boundary), boundary) == index * boundary + (boundary - 1),
            }
        }
    }

    quickcheck! {
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
                // If move_window returned None, the height must have been 0
                height == 0
            }
        }
    }
}
