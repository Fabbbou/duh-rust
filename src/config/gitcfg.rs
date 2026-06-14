//! Read/write a package's `gitconfig` file (git aliases) via `git2::Config`,
//! which preserves the file's formatting. The file is then wired into the user's
//! `~/.gitconfig` by [`crate::inject::gitinc`] on the next inject.

use crate::config::paths;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::PathBuf;

/// Validate a git alias name (e.g. `co`, `lg1`, `s-t`). Git stores it as the
/// variable part of `alias.<name>`, so it can't contain `.` or whitespace.
fn validate(name: &str) -> Result<()> {
    let ok = !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        bail!("invalid git alias name {name:?}: use letters, digits, '-', '_'");
    }
    Ok(())
}

/// Open (creating if needed) the package gitconfig as a single-file config.
fn open(pkg: &str) -> Result<(git2::Config, PathBuf)> {
    let path = paths::package_gitconfig(pkg)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        fs::write(&path, "").with_context(|| format!("creating {}", path.display()))?;
    }
    let cfg = git2::Config::open(&path).with_context(|| format!("opening {}", path.display()))?;
    Ok((cfg, path))
}

/// Set (add or update) a git alias in the package's gitconfig.
pub fn set_alias(pkg: &str, name: &str, value: &str) -> Result<()> {
    validate(name)?;
    let (mut cfg, _) = open(pkg)?;
    cfg.set_str(&format!("alias.{name}"), value)
        .with_context(|| format!("setting alias.{name}"))?;
    Ok(())
}

/// Remove a git alias from the package's gitconfig.
pub fn remove_alias(pkg: &str, name: &str) -> Result<()> {
    validate(name)?;
    let path = paths::package_gitconfig(pkg)?;
    if !path.exists() {
        bail!("no gitconfig in package {pkg}");
    }
    let mut cfg = git2::Config::open(&path)?;
    cfg.remove(&format!("alias.{name}"))
        .with_context(|| format!("no git alias {name:?} in package {pkg}"))?;
    Ok(())
}

/// List a package's git aliases as (name, value), sorted; empty if no gitconfig.
pub fn aliases(pkg: &str) -> Result<Vec<(String, String)>> {
    let path = paths::package_gitconfig(pkg)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let cfg = git2::Config::open(&path)?;
    let mut out = Vec::new();
    let mut entries = cfg.entries(Some("^alias\\."))?;
    while let Some(entry) = entries.next() {
        let entry = entry?;
        if let (Some(name), Some(value)) = (entry.name(), entry.value()) {
            let short = name.strip_prefix("alias.").unwrap_or(name).to_string();
            out.push((short, value.to_string()));
        }
    }
    out.sort();
    Ok(out)
}

/// Whether the package has a gitconfig file.
pub fn has_gitconfig(pkg: &str) -> bool {
    paths::package_gitconfig(pkg)
        .map(|p| p.exists())
        .unwrap_or(false)
}
