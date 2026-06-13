//! `duh pkg add|rm|ls|sync|push|enable|disable`

use crate::config::package::{self, Package};
use crate::config::paths;
use crate::config::prefs::Prefs;
use crate::git;
use anyhow::{bail, Context, Result};
use clap::Subcommand;
use std::fs;

#[derive(Subcommand)]
pub enum PkgCmd {
    /// Clone a remote package
    Add {
        url: String,
        /// Local name (defaults to the repo name)
        name: Option<String>,
    },
    /// Delete a local package
    Rm { name: String },
    /// List packages and their enabled state
    Ls,
    /// Pull updates for all enabled remote packages
    Sync,
    /// Commit and push local changes of a package
    Push { name: String },
    /// Enable a package for injection
    Enable { name: String },
    /// Disable a package
    Disable { name: String },
}

pub fn run(cmd: PkgCmd) -> Result<()> {
    match cmd {
        PkgCmd::Add { url, name } => add(&url, name),
        PkgCmd::Rm { name } => remove(&name),
        PkgCmd::Ls => list(),
        PkgCmd::Sync => sync(),
        PkgCmd::Push { name } => push(&name),
        PkgCmd::Enable { name } => set_enabled(&name, true),
        PkgCmd::Disable { name } => set_enabled(&name, false),
    }
}

fn derive_name(url: &str) -> String {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("package")
        .trim_end_matches(".git")
        .to_string()
}

fn add(url: &str, name: Option<String>) -> Result<()> {
    let name = name.unwrap_or_else(|| derive_name(url));
    paths::validate_package_name(&name)?;
    let dest = paths::package_dir(&name)?;
    if dest.exists() {
        bail!("package {name} already exists");
    }
    git::clone(url, &dest)?;
    warn_if_ships_functions(&name)?;

    let mut prefs = Prefs::load()?;
    prefs.enable(&name);
    prefs.save()?;
    println!("added and enabled package {name}");
    Ok(())
}

fn remove(name: &str) -> Result<()> {
    if name == paths::DEFAULT_PACKAGE {
        bail!("refusing to remove the default package");
    }
    let dir = paths::package_dir(name)?;
    if !dir.exists() {
        bail!("no package {name:?}");
    }
    fs::remove_dir_all(&dir).with_context(|| format!("removing {}", dir.display()))?;
    let mut prefs = Prefs::load()?;
    prefs.disable(name);
    prefs.save()?;
    println!("removed package {name}");
    Ok(())
}

fn list() -> Result<()> {
    let prefs = Prefs::load()?;
    let all = package::list_all()?;
    if all.is_empty() {
        println!("no packages");
        return Ok(());
    }
    for name in all {
        let enabled = prefs.packages.enabled.iter().any(|p| p == &name);
        let is_default = prefs.packages.default == name;
        let mark = if enabled { "x" } else { " " };
        let star = if is_default { " (default)" } else { "" };
        println!("[{mark}] {name}{star}");
    }
    Ok(())
}

fn sync() -> Result<()> {
    let prefs = Prefs::load()?;
    for name in prefs.enabled_existing()? {
        let pkg = Package::load(&name)?;
        if pkg.metadata.url_origin.is_empty() && !paths::package_dir(&name)?.join(".git").exists() {
            continue; // local-only package
        }
        let dir = paths::package_dir(&name)?;
        if dir.join(".git").exists() {
            match git::pull(&dir) {
                Ok(()) => println!("synced {name}"),
                Err(e) => eprintln!("skip {name}: {e:#}"),
            }
        }
    }
    Ok(())
}

fn push(name: &str) -> Result<()> {
    let dir = paths::package_dir(name)?;
    if !dir.join(".git").exists() {
        bail!("package {name} is not a git repo");
    }
    git::commit_and_push(&dir, "duh: update package")?;
    println!("pushed {name}");
    Ok(())
}

fn set_enabled(name: &str, enabled: bool) -> Result<()> {
    if !paths::package_dir(name)?.exists() {
        bail!("no package {name:?}");
    }
    if enabled {
        warn_if_ships_functions(name)?;
    }
    let mut prefs = Prefs::load()?;
    if enabled {
        prefs.enable(name);
    } else {
        prefs.disable(name);
    }
    prefs.save()?;
    println!("{} {name}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

/// Function bodies are injected into your shell VERBATIM (by design). For a
/// package from an untrusted remote, that is arbitrary code execution — warn so
/// the user knows enabling it runs that code on every prompt.
fn warn_if_ships_functions(name: &str) -> Result<()> {
    let funcs = Package::function_files(name)?;
    if !funcs.is_empty() {
        eprintln!(
            "warning: package {name} ships {} function file(s) that run in your shell \
             on every prompt. Review them before trusting this package:",
            funcs.len()
        );
        for f in &funcs {
            eprintln!("  {}", f.display());
        }
    }
    Ok(())
}
