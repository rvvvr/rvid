use std::{io::{stdin, stdout, Write, Read}, fs::File, path::PathBuf, env};

use editor::Editor;
use raw_tty::GuardMode;

pub mod tui;
pub mod editor;
pub mod piece_table;

fn main() -> anyhow::Result<()>{
    let mut stdin = stdin().guard_mode()?;
    stdin.set_raw_mode()?;
    let mut editor = Editor::new(stdin, PathBuf::from(env::args().nth(1).unwrap_or("file.txt".to_string())));
    editor.run()?;
    Ok(())
}
