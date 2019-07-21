use xim::{App, Config};

use docopt::Docopt;
use serde_derive::Deserialize;

const USAGE: &str = "
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
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct Args {
    arg_file: String,
}

// Translation of main::Args to a xim::Config
impl From<Args> for Config {
    fn from(args: Args) -> Self {
        Config {
            file: args.arg_file,
        }
    }
}

fn main() -> Result<(), Box<std::error::Error>> {
    // Parse arguments
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.version(Some(VERSION.into())).deserialize())
        .unwrap_or_else(|e| e.exit());

    // Run application
    App::new(args.into()).run()
}
