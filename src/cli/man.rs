//! `duh man` — render the man page (roff) to stdout.
//!
//! Packagers/users: `duh man > duh.1`. Shell completion needs no static files —
//! it's dynamic via `duh init` (`source <(COMPLETE=… duh)`).

use anyhow::Result;
use clap::CommandFactory;

pub fn run() -> Result<()> {
    let cmd = super::Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf)?;
    use std::io::Write;
    std::io::stdout().write_all(&buf)?;
    Ok(())
}
