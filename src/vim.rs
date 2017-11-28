// TODO: Merge with planned refactoring with Command-Pattern?
pub enum VimCommand {
    Quit,
    QuitWithoutSaving,
    Save,
    SaveAndQuit,
    Jump(usize),
}

impl VimCommand {
    pub fn parse(cmd: &str) -> Result<VimCommand, &'static str> {
        use self::VimCommand::*;

        match cmd {
            "q" => Ok(Quit),
            "q!" => Ok(QuitWithoutSaving),
            "w" => Ok(Save),
            "wq" | "x" => Ok(SaveAndQuit),
            offset => {
                // If none of the above commands, try to interpret as jump command...

                let (skip, base) = if offset.starts_with("0b") {
                    (2, 2)
                } else if offset.starts_with("08") {
                    (2, 8)
                } else if offset.starts_with("0x") {
                    (2, 16)
                } else {
                    (0, 10)
                };

                // ...and error out if no valid offset. (Proper parsing may be implemented in the future.)
                if let Ok(offset) = usize::from_str_radix(&offset[skip..], base) {
                    Ok(Jump(offset))
                } else {
                    Err("no such command")
                }
            },
        }
    }
}

#[derive(Clone)]
pub enum VimState {
    Normal,
    Insert1,
    Insert2(u8),
    Replace1,
    Replace2(u8),
    ReplaceMany1,
    ReplaceMany2(u8),
    Visual,
    Command(String),
}