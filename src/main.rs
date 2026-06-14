//! duh — inject your shell config (aliases, exports, functions) everywhere.

mod cli;
mod config;
mod git;
mod inject;
mod ui;

use anyhow::Result;
use clap::{CommandFactory, Parser};

fn main() {
    // Dynamic shell completion: when invoked with the COMPLETE env var, emit
    // candidates and exit. No-op otherwise. Must run before any stdout writes.
    clap_complete::CompleteEnv::with_factory(cli::Cli::command).complete();

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
