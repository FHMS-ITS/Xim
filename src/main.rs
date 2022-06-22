use docopt::Docopt;
use serde_derive::Deserialize;
use xim::{App, Args};

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
struct DocoptArgs {
    arg_file: String,
}

// Translation of `DocoptArgs` to `xim::Args`
impl From<DocoptArgs> for Args {
    fn from(args: DocoptArgs) -> Args {
        Args {
            file: args.arg_file,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse arguments
    let args: DocoptArgs = Docopt::new(USAGE)
        .and_then(|d| d.version(Some(VERSION.into())).deserialize())
        .unwrap_or_else(|e| e.exit());

    // Run application
    App::new(args.into()).run()
}
