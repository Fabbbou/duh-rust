//! `duh _internal …` — machine-only commands the shell wiring calls. Hidden from
//! `--help` and completion; you never type these. Wired by `duh init`:
//! `emit` runs at shell start (and on `duh-reload`), `hook` runs each prompt.

use crate::inject::cache;
use crate::inject::generator::{self, GenOptions};
use crate::ui;
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum InternalCmd {
    /// Per-prompt staleness check: stat-only, prints a reload command if stale.
    Hook,
    /// Regenerate the cache and print the script for `eval`.
    Emit {
        /// Suppress comments (used by the rc wiring).
        #[arg(long)]
        quiet: bool,
    },
}

pub fn run(cmd: InternalCmd) -> Result<()> {
    match cmd {
        InternalCmd::Hook => hook(),
        InternalCmd::Emit { quiet } => emit(quiet),
    }
}

/// Hot path: stat-only, RAW output (eval'd by the shell). No ui, no bootstrap.
fn hook() -> Result<()> {
    if cache::is_stale()? {
        println!("eval \"$(duh _internal emit --quiet)\"");
    }
    Ok(())
}

/// Regenerate the cache and print the script for `eval`.
fn emit(quiet: bool) -> Result<()> {
    let script = generator::generate(&GenOptions {
        quiet,
        ..Default::default()
    })?;
    cache::write(&script)?;

    // Sync per-package gitconfig includes into ~/.gitconfig (local only — never
    // over SSH). Errors go to STDERR so stdout stays clean for `eval`.
    match crate::inject::gitinc::sync_enabled() {
        Ok(added) if !added.is_empty() && !quiet => {
            for p in added {
                eprintln!("{}", ui::ok(&format!("git: included {}", p.display())));
            }
        }
        Err(e) => eprintln!("{}", ui::warn(&format!("gitconfig include sync: {e:#}"))),
        _ => {}
    }

    print!("{script}");
    Ok(())
}
