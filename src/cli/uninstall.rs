//! `duh uninstall` — remove the binary, cache, and optionally local packages.

use crate::config::paths;
use anyhow::{Context, Result};
use dialoguer::Confirm;
use std::fs;
use std::path::Path;

pub fn run(yes: bool, purge: bool) -> Result<()> {
    // Confirm the uninstall itself unless --yes/--purge.
    if !yes && !purge && !confirm("Uninstall duh?", false)? {
        println!("aborted");
        return Ok(());
    }

    // Decide whether to wipe user data (packages + config).
    //   --purge        → yes, no prompt
    //   --yes (only)   → no, keep data (safe non-interactive default)
    //   interactive    → prompt
    let data = paths::data_dir()?;
    let config = paths::config_dir()?;
    let delete_data = if purge {
        true
    } else if yes {
        false
    } else {
        confirm(
            &format!(
                "Also delete all local packages and config?\n  {}\n  {}",
                data.display(),
                config.display()
            ),
            false,
        )?
    };

    // Cache is regenerated state — always safe to remove.
    remove_dir(&paths::cache_dir()?, "cache");

    if delete_data {
        remove_dir(&data, "packages");
        remove_dir(&config, "config");
    } else {
        println!("kept packages + config ({})", data.display());
    }

    // Remove the binary last (on Unix, unlinking the running executable is fine).
    // `DUH_KEEP_BINARY` lets the test suite exercise the rest without self-deleting.
    if std::env::var_os("DUH_KEEP_BINARY").is_some() {
        println!("duh uninstalled (binary kept).");
        return Ok(());
    }
    match std::env::current_exe() {
        Ok(exe) => match fs::remove_file(&exe) {
            Ok(()) => println!("removed binary {}", exe.display()),
            Err(e) => eprintln!(
                "could not remove binary {}: {e}\n  delete it manually",
                exe.display()
            ),
        },
        Err(e) => eprintln!("could not locate binary: {e}"),
    }

    println!("duh uninstalled.");
    if !delete_data {
        println!(
            "Re-install anytime; your packages are still in {}",
            data.display()
        );
    }
    println!("Remember to remove the `eval \"$(duh init ...)\"` line from your shell rc.");
    Ok(())
}

/// Remove a directory tree if present, reporting the outcome.
fn remove_dir(dir: &Path, label: &str) {
    if !dir.exists() {
        return;
    }
    match fs::remove_dir_all(dir).with_context(|| format!("removing {}", dir.display())) {
        Ok(()) => println!("removed {label} ({})", dir.display()),
        Err(e) => eprintln!("warning: {e:#}"),
    }
}

/// Interactive yes/no prompt; defaults to `default` on non-tty.
fn confirm(prompt: &str, default: bool) -> Result<bool> {
    Ok(Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()
        .unwrap_or(default))
}
