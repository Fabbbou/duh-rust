//! `duh open [package]` — open a package folder with the configured tool.

use crate::config::paths;
use crate::config::prefs::Prefs;
use anyhow::{bail, Context, Result};
use std::process::Command;

pub fn run(package: Option<String>) -> Result<()> {
    let prefs = Prefs::load()?;
    let name = package.unwrap_or_else(|| prefs.packages.default.clone());
    let dir = paths::package_dir(&name)?; // validates name
    if !dir.exists() {
        bail!("no package {name:?} (see `duh pkg ls`)");
    }

    // Tool resolution: configured [tools] open → $EDITOR → $VISUAL → error.
    let tool = if !prefs.tools.open.is_empty() {
        prefs.tools.open.clone()
    } else if let Ok(e) = std::env::var("EDITOR") {
        e
    } else if let Ok(v) = std::env::var("VISUAL") {
        v
    } else {
        bail!(
            "no opener configured. Set one in prefs.toml:\n  [tools]\n  open = \"code\"\n\
             …or export $EDITOR."
        );
    };

    // tool is split on whitespace so values like "code -n" work; the dir is a
    // discrete arg (never a shell string), so the package path can't inject.
    let mut parts = tool.split_whitespace();
    let bin = parts.next().context("empty opener command")?;
    let status = Command::new(bin)
        .args(parts)
        .arg(&dir)
        .status()
        .with_context(|| format!("launching opener {bin:?}"))?;
    if !status.success() {
        bail!("opener {bin:?} exited with failure");
    }
    println!("opened package \"{name}\" ({}) with {tool}", dir.display());
    Ok(())
}
