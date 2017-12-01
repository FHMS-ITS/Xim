extern crate xim;
extern crate docopt;
#[macro_use]
extern crate serde_derive;

use docopt::Docopt;
use std::error::Error;
use xim::{App, Config};

const USAGE: &'static str = "
Xim

Usage:
  xim <file>
  xim (-h | --help)
  xim --version

Options:
  -h --help     Show this screen.
  --version     Show version.
";

// Get version from Cargo.toml
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct Args {
    arg_file: String,
}

// How to translate main::Args to a xim::Config?
impl From<Args> for Config {
    fn from(args: Args) -> Self {
        Config {
            file: args.arg_file,
        }
    }
}

fn run() -> Result<(), Box<Error>> {
    // Parse arguments
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.version(Some(VERSION.into())).deserialize())
        .unwrap_or_else(|e| e.exit());

    // Run application
    App::new(args.into()).run()
}

fn main() {
    run().unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        std::process::exit(1);
    });
    println!("\n"); // line-break after exiting
}