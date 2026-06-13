//! duh — inject your shell config (aliases, exports, functions) everywhere.

mod cli;
mod config;
mod git;
mod inject;

use anyhow::Result;
use clap::Parser;

fn main() {
    if let Err(e) = run() {
        eprintln!("duh: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = cli::Cli::parse();
    cli.dispatch()
}
