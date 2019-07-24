use std::cmp::min;

use clipboard::{ClipboardContext, ClipboardProvider};

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

pub fn save_to_clipboard(data: &[u8]) -> Result<String, String> {
    let cb: Result<ClipboardContext, _> = ClipboardProvider::new().map_err(|e| format!("{}", e));
    let mut cb = cb?;

    match cb.set_contents(hex::encode(data)) {
        Ok(_) => match data.len() {
            0 => Err("No data to copy".into()),
            1 => Ok(format!("Copied to clipboard ({})", hex::encode(&data[..1]))),
            _ => Ok(format!(
                "Copied to clipboard ({}...)",
                hex::encode(&data[..min(data.len(), 3)])
            )),
        },
        Err(e) => Err(format!("Failed copy to clipboard ({})", e)),
    }
}

pub fn read_from_clipboard() -> Result<Vec<u8>, String> {
    let cb: Result<ClipboardContext, _> = ClipboardProvider::new().map_err(|e| format!("{}", e));
    let mut cb = cb?;

    let data = cb.get_contents().map_err(|e| format!("{}", e))?;
    let data: String = data.chars().filter(|c| !c.is_whitespace()).collect();

    hex::decode(&data).map_err(|e| format!("{}", e))
}

pub fn offset_width(max: usize) -> u16 {
    format!("{:x}", max).len() as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn test_align(index: u16, random: u16, boundary: u16) -> bool {
        match boundary {
            0 => align(index, boundary) == index,
            1 => align(index, boundary) == index,
            _ => align(index * boundary + (random % boundary), boundary) == index * boundary,
        }
    }

    #[quickcheck]
    fn test_align_top(index: u16, random: u16, boundary: u16) -> bool {
        match boundary {
            0 => align_top(index, boundary) == index,
            1 => align_top(index, boundary) == index,
            _ => {
                align_top(index * boundary + (random % boundary), boundary)
                    == index * boundary + (boundary - 1)
            }
        }
    }

    #[quickcheck]
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
