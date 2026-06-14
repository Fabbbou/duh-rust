//! `duh inject`, `duh status [--hook]`.

use crate::config::prefs::Prefs;
use crate::inject::cache;
use crate::inject::generator::{self, GenOptions};
use anyhow::Result;

/// Regenerate the cache and print the script for `eval`.
pub fn inject(quiet: bool) -> Result<()> {
    let script = generator::generate(&GenOptions {
        quiet,
        ..Default::default()
    })?;
    cache::write(&script)?;
    print!("{script}");
    Ok(())
}

/// `duh status` — human summary; `duh status --hook` — stat-only reload trigger.
pub fn status(hook: bool) -> Result<()> {
    if hook {
        // Hot path: stat-only. Print a reload command only when stale.
        if cache::is_stale()? {
            println!("eval \"$(duh inject --quiet)\"");
        }
        return Ok(());
    }

    let c = generator::counts()?;
    let state = if cache::is_stale()? {
        "stale (run `duh-reload`, or it self-heals on the next prompt)"
    } else {
        "in sync"
    };
    println!(
        "duh: {} package(s), {} alias(es), {} export(s), {} function(s) — {}",
        c.packages, c.aliases, c.exports, c.functions, state
    );
    let default = Prefs::load()?.packages.default;
    println!("default package: {default}  (add/rm write here)");
    Ok(())
}
