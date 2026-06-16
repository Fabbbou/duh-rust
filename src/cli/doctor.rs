//! `duh doctor` — diagnose a duh setup. Exits 1 if any hard error is found.

use crate::config::package::{self, Package};
use crate::config::prefs::Prefs;
use crate::config::{conflicts, gitcfg, paths};
use crate::inject::cache;
use crate::ui;
use anyhow::Result;
use std::fs;

pub fn run() -> Result<()> {
    let mut errors = 0u32;
    let ok = |m: &str| println!("{} {m}", ui::mark(true));
    let warn = |m: &str| println!("{} {m}", ui::mark_warn());
    let mut bad = |m: &str| {
        println!("{} {m}", ui::mark(false));
        errors += 1;
    };

    let prefs = Prefs::load()?;
    let on_disk = package::list_all()?;

    // 1. shell wiring
    match shell_wired() {
        Some(rc) => ok(&format!("shell wired ({rc})")),
        None => warn("shell not wired — add `eval \"$(duh init)\"` to your rc"),
    }

    // 2. default package
    if on_disk.contains(&prefs.packages.default) {
        ok(&format!(
            "default package `{}` exists",
            prefs.packages.default
        ));
    } else {
        bad(&format!(
            "default package `{}` does not exist (run `duh use <pkg>`)",
            prefs.packages.default
        ));
    }

    // 3. enabled packages exist
    for p in &prefs.packages.enabled {
        if !on_disk.contains(p) {
            bad(&format!("enabled package `{p}` is missing on disk"));
        }
    }
    if prefs.packages.enabled.iter().all(|p| on_disk.contains(p)) {
        ok(&format!(
            "{} enabled package(s) all present",
            prefs.packages.enabled.len()
        ));
    }

    // 4. cache freshness
    if cache::is_stale()? {
        warn("cache is stale — open a new shell or run `duh-reload`");
    } else {
        ok("cache in sync");
    }

    // 5. gitconfig includes
    let gc_path = paths::git_config_path()?;
    let gc_text = fs::read_to_string(&gc_path).unwrap_or_default();
    for p in prefs.enabled_existing()? {
        if gitcfg::has_gitconfig(&p) {
            let inc = paths::package_gitconfig(&p)?.display().to_string();
            if gc_text.contains(&inc) {
                ok(&format!("git include present for `{p}`"));
            } else {
                warn(&format!(
                    "package `{p}` has a gitconfig not yet included — run `duh inject`"
                ));
            }
        }
    }

    // 6. conflicts across enabled packages
    for c in conflicts::find()? {
        warn(&format!(
            "{} `{}` defined by {} — `{}` wins",
            c.kind,
            c.name,
            c.packages.join(", "),
            c.winner
        ));
    }

    // 7. function lint warnings
    for p in prefs.enabled_existing()? {
        for f in Package::function_files(&p)? {
            for w in package::function_lint(&f) {
                warn(&format!("lint: {w}"));
            }
        }
    }

    println!();
    if errors == 0 {
        println!("{}", ui::ok("no problems found"));
        Ok(())
    } else {
        std::process::exit(1);
    }
}

/// Detect whether a shell rc references duh's injection. Returns the rc name.
fn shell_wired() -> Option<String> {
    let home = directories::BaseDirs::new()?.home_dir().to_path_buf();
    for rc in [".bashrc", ".zshrc", ".bash_profile", ".profile"] {
        if let Ok(text) = fs::read_to_string(home.join(rc)) {
            if text.contains("duh inject") || text.contains("duh init") {
                return Some(rc.to_string());
            }
        }
    }
    None
}
