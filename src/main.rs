extern crate xim;
use xim::model::*;
use xim::view::*;
use xim::controller::*;

extern crate docopt;
#[macro_use] extern crate serde_derive;
use docopt::Docopt;

extern crate chan;
extern crate chan_signal;
use chan_signal::{Signal, notify};

extern crate termion;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;

use std::error::Error;
use std::io::{Write, stdout, stdin};
use std::sync::mpsc::sync_channel;
use std::thread;

use std::rc::Rc;
use std::cell::RefCell;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

const USAGE: &'static str = "
Usage:
  xim <file>
  xim (-h | --help)
  xim --version

Options:
  -h --help     Show this screen.
  --version     Show version.
";

#[derive(Deserialize)]
struct Args {
    arg_file: String,
}

struct Config {
    pub file: String,
}

impl From<Args> for Config {
    fn from(args: Args) -> Self {
        Config {
            file: args.arg_file,
        }
    }
}

enum Event {
    Key(Key),
    Resize((u16, u16)),
}

fn setup_terminal(stdout: RawStdout) -> Result<(), Box<Error>> {
    let mut stdout = stdout.borrow_mut();
    write!(stdout, "{}", termion::cursor::Hide)?;
    write!(stdout, "{}", termion::clear::All)?;
    stdout.flush()?;
    Ok(())
}

fn teardown_terminal(stdout: RawStdout) -> Result<(), Box<Error>> {
    let mut stdout = stdout.borrow_mut();
    write!(stdout, "{}", termion::clear::All)?;
    write!(stdout, "{}", termion::cursor::Show)?;
    stdout.flush()?;
    Ok(())
}

fn run() -> Result<(), Box<Error>> {
    let config: Config = {
        let args: Args = Docopt::new(USAGE)
            .and_then(|d| d.version(Some(VERSION.into())).deserialize())
            .unwrap_or_else(|e| e.exit());

        args.into()
    };

    let (stdin, stdout) = {
        let stdin = stdin();
        let stdout = Rc::new(RefCell::new(AlternateScreen::from(stdout().into_raw_mode()?)));
        (stdin, stdout)
    };

    let events = {
        // Create event channel
        let (send, recv) = sync_channel(0);

        // Register window changed event
        let signal_winch = notify(&[Signal::WINCH]);

        // Receive window changed events
        let send_1 = send.clone();
        thread::spawn(move || {
            for _ in signal_winch.iter() {
                send_1.send(Event::Resize(termion::terminal_size().unwrap())).unwrap();
            }
        });

        // Receive keypress events
        let send_2 = send.clone();
        thread::spawn(move || {
            for c in stdin.keys() {
                send_2.send(Event::Key(c.unwrap())).unwrap();
            }
        });

        recv
    };

    let mut ctrl = {
        let model = Model::new();

        let view = {
            let hex_view = HexView::new(stdout.clone());
            let status_view = StatusView::new(stdout.clone());

            View {
                hex_view,
                status_view,
            }
        };

        Controller::new(model, view)
    };

    setup_terminal(stdout.clone())?;

    ctrl.resize_view(termion::terminal_size()?);
    ctrl.open(&config.file);
    ctrl.update_view();

    for event in events.iter() {
        match event {
            Event::Resize(new_size) => {
                ctrl.resize_view(new_size);
            },
            Event::Key(k) => {
                if !ctrl.transition(k) {
                    break
                }
            }
        }

        ctrl.update_view();
    }

    teardown_terminal(stdout.clone())?;

    Ok(())
}

fn main() {
    run().unwrap_or_else(|e| {
        eprintln!("error: {:?}", e);
        std::process::exit(1);
    });
    println!("\n"); // line-break after exiting
}