//! duh — inject your shell config (aliases, exports, functions) everywhere.

mod cli;
mod config;
mod git;
mod inject;
mod ui;

use anyhow::Result;
use clap::Parser;

fn main() {
    // Restore default SIGPIPE so `duh … | head` exits quietly instead of
    // panicking on a broken pipe (Rust sets SIGPIPE to SIG_IGN by default).
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    if let Err(e) = run() {
        eprintln!("{}", ui::err(&format!("{e:#}")));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = cli::Cli::parse();
    cli.dispatch()
}
