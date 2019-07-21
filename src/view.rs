use crate::{Ascii, align, align_top, Caret, move_window, model::Model, RawStdout};

use std::{
    cmp::{min, max},
    io::{Write, Result as IoResult},
    mem::swap
};

use termion::{
    clear::{All as ClearAll, CurrentLine as ClearCurrentLine},
    color::{Fg, Red, Reset as ColorReset},
    cursor::Goto,
    style::{Bold, Invert, NoInvert, Underline, Reset as StyleReset},
};

fn chunks_indices(mut start: u16, end: u16, size: u16) -> Vec<(u16, u16)> {
    let mut result = Vec::with_capacity(((end - start) / 16) as usize);

    while start <= end {
        result.push((start, min(start + size - 1, end)));
        start += size;
    }

    result
}

pub fn range_to_marker(mut start: u16, mut end: u16) -> Vec<(u16, u16, u16)> {
    if start > end {
        swap(&mut start, &mut end);
    };

    let lines = (start/16..end/16 + 1).collect::<Vec<_>>();
    let mut spans = chunks_indices(align(start, 16), align_top(end, 16), 16);
    spans.first_mut().unwrap().0 += start;
    spans.last_mut().unwrap().1 = end % 16;

    lines.iter().zip(spans.iter()).map(|(line, &(x, y))| (*line, x % 16, y % 16)).collect()
}

pub struct DrawArea {
    pub origin: (u16, u16),
    pub dimens: (u16, u16),
}

pub struct View {
    area: DrawArea,
    stdout: RawStdout,
    pub hex_view: HexView,
    pub status_view: StatusView,
}

impl View {
    pub fn new(stdout: RawStdout) -> View {
        let hex_view = HexView::new(stdout.clone());
        let status_view = StatusView::new(stdout.clone());

        View {
            area: DrawArea {
                origin: (1, 1),
                dimens: (16, 16),
            },
            stdout: stdout,
            hex_view,
            status_view,
        }
    }

    pub fn draw(&self, model: &Model) -> IoResult<()> {
        // limit scope of stdout here, because hex_view and status_view have their own reference.
        {
            let mut stdout = self.stdout.borrow_mut();

            // TODO: Better redraw only the dirty parts (ClearAll causes the flickering.)
            write!(stdout, "{}", ClearAll).unwrap();

            write!(stdout, "{}", Fg(Red))?;
            for line in 1..(self.area.dimens.1 - 1) {
                write!(stdout, "{}~", Goto(1, line))?;
            }
            write!(stdout, "{}", Fg(ColorReset))?;
        }

        self.hex_view.draw(model)?;
        self.status_view.draw()?;

        Ok(())
    }

    pub fn set_area(&mut self, area: DrawArea) {
        let DrawArea { origin: (x, y), dimens: (w, h) } = area;

        // Set mimimum width/height to avoid overfow
        let (w, h) = (max(w, 75), max(h, 4));

        self.area = DrawArea {
            origin: (x, y),
            dimens: (w, h),
        };

        self.hex_view.set_area(DrawArea {
            origin: (x, y),
            dimens: (w, h - 3),
        });

        self.status_view.set_area(DrawArea {
            origin: (x, y + h - 2),
            dimens: (w, 2),
        });
    }
}

pub struct HexView {
    scroll_start: usize,
    area: DrawArea,
    stdout: RawStdout,
}

impl HexView {
    pub fn new(stdout: RawStdout) -> HexView {
        HexView {
            scroll_start: 0,
            area: DrawArea {
                origin: (1, 1),
                dimens: (16, 16),
            },
            stdout: stdout,
        }
    }

    pub fn set_area(&mut self, area: DrawArea) {
        self.area = area;
    }

    pub fn draw(&self, model: &Model) -> IoResult<()> {
        let mut stdout = self.stdout.borrow_mut();

        let offset_width = format!("{:x}", model.buffer.len()).len();

        let DrawArea { origin: (x, y), dimens: (w, h) } = self.area;
        let offset_area = DrawArea { origin: (x, y+1), dimens: (offset_width as u16, h), };
        let hex_area = DrawArea {origin: (offset_area.origin.0 + offset_area.dimens.0 + 2, y+1), dimens: (16*2 + 15, h), };
        let ascii_area = DrawArea { origin: (hex_area.origin.0 + hex_area.dimens.0 + 2, y+1), dimens: (16, h) };

        if model.buffer.is_empty() {
            let msg = "empty file: go into insert mode and insert some bytes";
            write!(stdout, "{}{}", Goto(w / 2 - (msg.len() as u16 / 2), h / 2), msg).unwrap();

            return Ok(());
        }

        // Draw indices
        write!(stdout, "{}", Fg(Red))?;
        write!(stdout, "{}{}", Goto(offset_width as u16 + 4, 1), "0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f")?;
        write!(stdout, "{}", Fg(ColorReset))?;

        for (line, chunk) in model.buffer[self.scroll_start..].chunks(16).take(h as usize).enumerate() {
            let offset = line * 16;
            let line = line as u16;

            // Draw offsets
            write!(stdout, "{}{}{:0width$x}: {}", Goto(offset_area.origin.0, offset_area.origin.1 + line), Fg(Red), offset + self.scroll_start, Fg(ColorReset), width=offset_width).unwrap();

            // Draw hex values
            write!(stdout, "{}", Goto(hex_area.origin.0, hex_area.origin.1 + line)).unwrap();
            for byte in chunk {
                write!(stdout, "{:02x} ", byte).unwrap();
            }

            // Draw ascii values
            write!(stdout, "{}", Goto(ascii_area.origin.0, ascii_area.origin.1 + line)).unwrap();
            for byte in chunk {
                write!(stdout, "{}", byte.to_printable()).unwrap();
            }
        }

        // Draw Caret
        match model.caret {
            Caret::Index(index) => {
                let index = usize::from(index);
                write!(stdout, "{}{}", Goto(hex_area.origin.0 + ((index % 16) as u16) * 3 - 1, hex_area.origin.1 + ((index - self.scroll_start) / 16) as u16), "|").unwrap();

                let value = if index < model.buffer.len() {
                    model.buffer[index].to_printable()
                } else {
                    ' '
                };

                write!(stdout, "{}{}{}{}", Goto(ascii_area.origin.0 + ((index % 16) as u16), ascii_area.origin.1 + ((index - self.scroll_start) / 16) as u16), Underline, value, StyleReset).unwrap();
            },
            Caret::Offset(index) => {
                let index = usize::from(index);
                let byte = model.buffer[index];

                write!(stdout, "{}{}{:02x}{}", Goto(hex_area.origin.0 + ((index % 16) as u16) * 3, hex_area.origin.1 + ((index - self.scroll_start) / 16) as u16), Invert, byte, StyleReset).unwrap();
                write!(stdout, "{}{}{}{}", Goto(ascii_area.origin.0 + ((index % 16) as u16), ascii_area.origin.1 + ((index - self.scroll_start) / 16) as u16), Underline, byte.to_printable(), StyleReset).unwrap();
            },
            Caret::Replace(index) => {
                let index = usize::from(index);
                let byte = model.buffer[index];

                write!(stdout, "{}{}{:02x}{}", Goto(hex_area.origin.0 + ((index % 16) as u16) * 3, hex_area.origin.1 + ((index - self.scroll_start) / 16) as u16), Underline, byte, StyleReset).unwrap();
                write!(stdout, "{}{}{}{}", Goto(ascii_area.origin.0 + ((index % 16) as u16), ascii_area.origin.1 + ((index - self.scroll_start) / 16) as u16), Underline, byte.to_printable(), StyleReset).unwrap();
            },
            Caret::Visual(start, end) => {
                let start = usize::from(start);
                let end = usize::from(end);
                let rel_start = (start.saturating_sub(self.scroll_start)) as u16;
                let rel_end = (end.saturating_sub(self.scroll_start)) as u16;

                let lines = range_to_marker(rel_start, rel_end);

                for &(line, s, e) in lines.iter().take(h as usize) {
                    for no in s..e {
                        let byte = model.buffer[no as usize + line as usize *16 + self.scroll_start];
                        write!(stdout, "{}{}{:02x} {}", Goto(hex_area.origin.0 + no * 3, hex_area.origin.1 + line), Invert, byte, StyleReset).unwrap();
                        write!(stdout, "{}{}{}{}", Goto(ascii_area.origin.0 + no, ascii_area.origin.1 + line), Underline, byte.to_printable(), StyleReset).unwrap();
                    }
                    let byte = model.buffer[e as usize + line as usize * 16 + self.scroll_start];
                    write!(stdout, "{}{}{:02x}{}", Goto(hex_area.origin.0 + e * 3, hex_area.origin.1 + line), Invert, byte, StyleReset).unwrap();
                    write!(stdout, "{}{}{}{}", Goto(ascii_area.origin.0 + e, ascii_area.origin.1 + line), Underline, byte.to_printable(), StyleReset).unwrap();
                }

                let byte = model.buffer[end];
                write!(stdout, "{}{}{}{:02x}{}", Goto(hex_area.origin.0 + ((end % 16) as u16) * 3, hex_area.origin.1 + ((end - self.scroll_start) / 16) as u16), Invert, Bold, byte, StyleReset).unwrap();
            },
        }

        Ok(())
    }

    pub fn scroll_to(&mut self, index: usize) {
        let DrawArea { origin: (_, _), dimens: (_, h) } = self.area;

        let start = self.scroll_start / 16;
        let index = index / 16;

        self.scroll_start = move_window(start, h as usize, index).unwrap() * 16;
    }
}

pub struct StatusView {
    pub head: String,
    pub body: String,
    pub index: usize,
    pub area: DrawArea,
    stdout: RawStdout,
}

impl StatusView {
    pub fn new(stdout: RawStdout) -> StatusView {
        StatusView {
            head: "".into(),
            body: "".into(),
            index: 0,
            area: DrawArea {
                origin: (1, 1),
                dimens: (16, 2),
            },
            stdout: stdout,
        }
    }

    pub fn set_head(&mut self, text: &str) {
        self.head = text.into();
    }

    pub fn set_body(&mut self, text: &str) {
        self.body = text.into();
    }

    pub fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    pub fn set_area(&mut self, area: DrawArea) {
        self.area = area;
    }

    pub fn draw(&self) -> IoResult<()> {
        let mut stdout = self.stdout.borrow_mut();

        let DrawArea { origin: (x, y), dimens: (w, _) } = self.area;

        write!(stdout, "{}{}{}{}{}", Goto(x, y), ClearCurrentLine, Invert, format!("{:<pad$}", self.head, pad=(w as usize)), NoInvert)?;
        write!(stdout, "{}{}{}", Goto(x, y + 1), ClearCurrentLine, self.body)?;
        let offset_msg = format!("0x{:x} ({})", self.index, self.index);
        write!(stdout, "{}{}", Goto(x + w/2 - (offset_msg.len() as u16 / 2), y + 1), offset_msg)?;

        stdout.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
}