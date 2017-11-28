use super::model::Model;
use super::{Caret, char_to_ascii_printable, range_to_marker};

use std::io::{Write, Stdout};
use termion;
use termion::color;
use termion::cursor::Goto;
use termion::raw::RawTerminal;
use std::cell::RefCell;
use std::rc::Rc;
use termion::screen::AlternateScreen;

pub type RawStdout = Rc<RefCell<AlternateScreen<RawTerminal<Stdout>>>>;

pub trait Draw {
    fn draw(&self, stdout: &mut RawStdout, area: DrawArea);
}

pub struct DrawArea {
    pub origin: (u16, u16),
    pub dimens: (u16, u16),
}

pub struct View {
    pub hex_view: HexView,
    pub status_view: StatusView,
}

impl View {
    pub fn draw(&self, model: &Model) {
        self.hex_view.draw(model);
        self.status_view.draw();
    }

    pub fn set_area(&mut self, area: DrawArea) {
        let DrawArea { origin: (x, y), dimens: (w, h) } = area;

        self.hex_view.set_area(DrawArea {
            origin: (x, y),
            dimens: (w, h - 3),
        });

        self.status_view.set_area(DrawArea {
            origin: (x, y + h - 2),
            dimens: (w, 1),
        });
    }
}

pub struct HexView {
    scroll_pos: usize,
    area: DrawArea,
    stdout: RawStdout,
}

impl HexView {
    pub fn new(stdout: RawStdout) -> HexView {
        HexView {
            scroll_pos: 0,
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

    pub fn scroll_to(&mut self, index: usize) {
        let DrawArea { origin: (_, _), dimens: (_, h) } = self.area;

        let index = index / 16;
        let start = self.scroll_pos / 16;
        let end = start + (h as usize - 1);

        if index < start {
            self.scroll_pos = index * 16;
        } else if index > end {
            self.scroll_pos = (index - (h as usize - 1)) * 16;
        }
    }

    pub fn draw(&self, model: &Model) {
        let mut stdout = self.stdout.borrow_mut();

        let DrawArea { origin: (x, y), dimens: (w, h) } = self.area;
        let offset_area = DrawArea { origin: (x, y+1), dimens: (8, h), };
        let hex_area = DrawArea {origin: (offset_area.origin.0 + offset_area.dimens.0 + 2, y+1), dimens: (16*2 + 15, h), };
        let ascii_area = DrawArea { origin: (hex_area.origin.0 + hex_area.dimens.0 + 2, y+1), dimens: (16, h) };

        // FIXME: DEBUG
        write!(stdout, "{}", termion::clear::All).unwrap();

        if model.buffer.len() == 0 {
            let msg = "empty file: go into insert mode and insert some bytes";
            write!(stdout, "{}{}", Goto(w / 2 - (msg.len() as u16 / 2), h / 2), msg).unwrap();
            return;
        }

        // Draw Indices
        write!(stdout, "{}{}{}{}", Goto(1, 1), color::Fg(color::Red), "~          0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f", color::Fg(color::Reset)).unwrap();

        for (line, chunk) in model.buffer[self.scroll_pos..].chunks(16).take(h as usize).enumerate() {
            // Draw Offset
            write!(stdout, "{}{}{:08x}: {}", Goto(offset_area.origin.0, offset_area.origin.1 + line as u16), color::Fg(color::Red), (line as usize) * 16 + self.scroll_pos, color::Fg(color::Reset)).unwrap();

            // Draw Hex Line
            write!(stdout, "{}", Goto(hex_area.origin.0, hex_area.origin.1 + line as u16)).unwrap();
            for byte in chunk {
                write!(stdout, "{:02x} ", byte).unwrap();
            }

            // Draw Ascii Line
            write!(stdout, "{}", Goto(ascii_area.origin.0, ascii_area.origin.1 + line as u16)).unwrap();
            for byte in chunk {
                write!(stdout, "{}", char_to_ascii_printable(*byte)).unwrap();
            }
        }

        // Draw Tildes
        for line in (model.buffer[self.scroll_pos..].chunks(16).take(h as usize).len() as u16)..(offset_area.dimens.1 as u16) {
            write!(stdout, "{}{}{}~{}", Goto(offset_area.origin.0, offset_area.origin.1 + line as u16), termion::clear::CurrentLine, color::Fg(color::Red), color::Fg(color::Reset)).unwrap();
        }

        // Draw Caret
        match model.caret {
            Caret::Index(index) => {
                let index = usize::from(index);
                write!(stdout, "{}{}", Goto(hex_area.origin.0 + ((index % 16) as u16) * 3 - 1, hex_area.origin.1 + ((index - self.scroll_pos) / 16) as u16), "|").unwrap();

                let value = if index < model.buffer.len() {
                    char_to_ascii_printable(model.buffer[index])
                } else {
                    ' '
                };
                write!(stdout, "{}{}{}{}", Goto(ascii_area.origin.0 + ((index % 16) as u16), ascii_area.origin.1 + ((index - self.scroll_pos) / 16) as u16), termion::style::Underline, value, termion::style::Reset).unwrap();
            },
            Caret::Offset(index) => {
                let index = usize::from(index);
                let byte = model.buffer[index];
                write!(stdout, "{}{}{:02x}{}", Goto(hex_area.origin.0 + ((index % 16) as u16) * 3, hex_area.origin.1 + ((index - self.scroll_pos) / 16) as u16), termion::style::Invert, byte, termion::style::Reset).unwrap();
                write!(stdout, "{}{}{}{}", Goto(ascii_area.origin.0 + ((index % 16) as u16), ascii_area.origin.1 + ((index - self.scroll_pos) / 16) as u16), termion::style::Underline, char_to_ascii_printable(byte), termion::style::Reset).unwrap();
            },
            Caret::Replace(index) => {
                let index = usize::from(index);
                let byte = model.buffer[index];
                write!(stdout, "{}{}{:02x}{}", Goto(hex_area.origin.0 + ((index % 16) as u16) * 3, hex_area.origin.1 + ((index - self.scroll_pos) / 16) as u16), termion::style::Underline, byte, termion::style::Reset).unwrap();
                write!(stdout, "{}{}{}{}", Goto(ascii_area.origin.0 + ((index % 16) as u16), ascii_area.origin.1 + ((index - self.scroll_pos) / 16) as u16), termion::style::Underline, char_to_ascii_printable(byte), termion::style::Reset).unwrap();
            },
            Caret::Visual(start, end) => {
                let start = usize::from(start);
                let end = usize::from(end);
                let rel_start = (start - self.scroll_pos) as u16;
                let rel_end = (end - self.scroll_pos) as u16;

                let lines = range_to_marker(rel_start, rel_end);

                for (line, s, e) in lines {
                    for no in s..e {
                        let byte = model.buffer[no as usize + line as usize *16 + self.scroll_pos];
                        write!(stdout, "{}{}{:02x} {}", Goto(hex_area.origin.0 + no * 3, hex_area.origin.1 + line), termion::style::Invert, byte, termion::style::Reset).unwrap();
                        write!(stdout, "{}{}{}{}", Goto(ascii_area.origin.0 + no, ascii_area.origin.1 + line), termion::style::Underline, char_to_ascii_printable(byte), termion::style::Reset).unwrap();
                    }
                    let byte = model.buffer[e as usize + line as usize * 16 + self.scroll_pos];
                    write!(stdout, "{}{}{:02x}{}", Goto(hex_area.origin.0 + e * 3, hex_area.origin.1 + line), termion::style::Invert, byte, termion::style::Reset).unwrap();
                    write!(stdout, "{}{}{}{}", Goto(ascii_area.origin.0 + e, ascii_area.origin.1 + line), termion::style::Underline, char_to_ascii_printable(byte), termion::style::Reset).unwrap();
                }

                let byte = model.buffer[end];
                write!(stdout, "{}{}{}{:02x}{}", Goto(hex_area.origin.0 + ((end % 16) as u16) * 3, hex_area.origin.1 + ((end - self.scroll_pos) / 16) as u16), termion::style::Invert, termion::style::Bold, byte, termion::style::Reset).unwrap();
            },
        }
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

    pub fn draw(&self) {
        let mut stdout = self.stdout.borrow_mut();

        use termion::cursor::Goto;
        use termion::clear::CurrentLine;
        use termion::style::{Invert, NoInvert};

        let DrawArea { origin: (x, y), dimens: (w, _) } = self.area;

        write!(stdout, "{}{}{}{}{}", Goto(x, y), CurrentLine, Invert, format!("{:<pad$}", self.head, pad=(w as usize)), NoInvert).unwrap();
        write!(stdout, "{}{}{}", Goto(x, y + 1), CurrentLine, self.body).unwrap();
        let offset_msg = format!("0x{:x} ({})", self.index, self.index);
        write!(stdout, "{}{}", Goto(x + w/2 - (offset_msg.len() as u16 / 2), y + 1), offset_msg).unwrap();

        stdout.flush().unwrap();
    }
}