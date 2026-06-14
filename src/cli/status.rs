//! `duh inject`, `duh status [--hook]`.

use crate::config::prefs::Prefs;
use crate::inject::cache;
use crate::inject::generator::{self, GenOptions};
use crate::ui;
use anyhow::Result;

/// Regenerate the cache and print the script for `eval`.
pub fn inject(quiet: bool) -> Result<()> {
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

/// `duh status` — human summary; `duh status --hook` — stat-only reload trigger.
pub fn status(hook: bool, json: bool) -> Result<()> {
    if hook {
        // Hot path: stat-only, RAW output (eval'd by the shell). No ui, no json.
        if cache::is_stale()? {
            println!("eval \"$(duh inject --quiet)\"");
        }
        return Ok(());
    }

    let c = generator::counts()?;
    let in_sync = !cache::is_stale()?;
    let default = Prefs::load()?.packages.default;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "packages": c.packages,
                "aliases": c.aliases,
                "exports": c.exports,
                "functions": c.functions,
                "in_sync": in_sync,
                "default": default,
            }))?
        );
        return Ok(());
    }

    let state = if in_sync {
        ui::state("in sync", true)
    } else {
        ui::state("stale", false)
    };
    println!(
        "{} {} package(s), {} alias(es), {} export(s), {} function(s) — {}",
        ui::dot(),
        c.packages,
        c.aliases,
        c.exports,
        c.functions,
        state
    );
    if !in_sync {
        println!(
            "  {}",
            ui::dim("run `duh-reload`, or it self-heals on the next prompt")
        );
    }
    println!(
        "{} default package: {}  {}",
        ui::dim("·"),
        ui::header(&default),
        ui::dim("(add/rm write here)")
    );
    Ok(())
}
