//! Package lifecycle operations, shared by the `create`/`delete`/`get` verbs and
//! the flat lifecycle verbs (`enable`/`disable`/`rename`/`sync`/`push`/`export`/
//! `import`). No clap types here — just plain functions over package state.

use crate::config::package::{self, Package};
use crate::config::paths;
use crate::config::prefs::Prefs;
use crate::git;
use anyhow::{bail, Context, Result};
use std::fs;

pub fn rename(old: &str, new: &str) -> Result<()> {
    let from = paths::package_dir(old)?;
    let to = paths::package_dir(new)?;
    if !from.exists() {
        bail!("no package {old:?}");
    }
    if to.exists() {
        bail!("package {new} already exists");
    }
    fs::rename(&from, &to).with_context(|| format!("renaming {old} → {new}"))?;
    let mut prefs = Prefs::load()?;
    for p in prefs.packages.enabled.iter_mut() {
        if p == old {
            *p = new.to_string();
        }
    }
    if prefs.packages.default == old {
        prefs.packages.default = new.to_string();
    }
    prefs.save()?;
    println!(
        "{}",
        crate::ui::ok(&format!("renamed {old} → {}", crate::ui::header(new)))
    );
    Ok(())
}

pub fn export(name: &str, out: Option<String>) -> Result<()> {
    let dir = paths::package_dir(name)?;
    if !dir.exists() {
        bail!("no package {name:?}");
    }
    let out = out.unwrap_or_else(|| format!("duh-{name}.tar.gz"));
    // tar -C <packages_dir> -czf <out> <name>  (archive holds the package dir).
    let parent = paths::packages_dir()?;
    let status = std::process::Command::new("tar")
        .arg("-C")
        .arg(&parent)
        .arg("-czf")
        .arg(&out)
        .arg(name)
        .status()
        .context("running tar (is it installed?)")?;
    if !status.success() {
        bail!("tar export failed");
    }
    println!("{}", crate::ui::ok(&format!("exported {name} → {out}")));
    Ok(())
}

pub fn import(file: &str, name: Option<String>) -> Result<()> {
    if !std::path::Path::new(file).exists() {
        bail!("no such file: {file}");
    }
    // Discover the top-level dir name in the archive (the package name).
    let listing = std::process::Command::new("tar")
        .arg("-tzf")
        .arg(file)
        .output()
        .context("running tar")?;
    if !listing.status.success() {
        bail!("could not read archive {file}");
    }
    let archived = String::from_utf8_lossy(&listing.stdout)
        .lines()
        .next()
        .map(|l| {
            l.trim_end_matches('/')
                .split('/')
                .next()
                .unwrap_or("")
                .to_string()
        })
        .unwrap_or_default();
    if archived.is_empty() {
        bail!("archive {file} has no package directory");
    }
    paths::validate_package_name(&archived)?;
    let target = name.unwrap_or_else(|| archived.clone());
    paths::validate_package_name(&target)?;
    if paths::package_dir(&target)?.exists() {
        bail!("package {target} already exists");
    }

    let parent = paths::packages_dir()?;
    fs::create_dir_all(&parent)?;
    let status = std::process::Command::new("tar")
        .arg("-C")
        .arg(&parent)
        .arg("-xzf")
        .arg(file)
        .status()
        .context("running tar")?;
    if !status.success() {
        bail!("tar import failed");
    }
    // Archive extracts to its own name; rename to the requested target if different.
    if target != archived {
        if paths::package_dir(&archived)?.exists() {
            fs::rename(paths::package_dir(&archived)?, paths::package_dir(&target)?)?;
        } else {
            bail!("archive did not contain a package named {archived:?}");
        }
    } else if !paths::package_dir(&target)?.exists() {
        bail!("archive did not contain a package named {target:?}");
    }
    let mut prefs = Prefs::load()?;
    prefs.enable(&target);
    prefs.save()?;
    println!(
        "{}",
        crate::ui::ok(&format!(
            "imported and enabled {}",
            crate::ui::header(&target)
        ))
    );
    Ok(())
}

/// Create a new empty local package and enable it.
pub fn create_empty(name: &str) -> Result<()> {
    paths::validate_package_name(name)?;
    if paths::package_dir(name)?.exists() {
        bail!("package {name} already exists");
    }
    let mut pkg = Package::default();
    pkg.metadata.name_origin = name.to_string();
    pkg.save(name)?;
    fs::create_dir_all(paths::package_functions_dir(name)?)?;

    let mut prefs = Prefs::load()?;
    prefs.enable(name);
    prefs.save()?;
    println!(
        "{}",
        crate::ui::ok(&format!(
            "created and enabled package {}",
            crate::ui::header(name)
        ))
    );
    Ok(())
}

fn derive_name(url: &str) -> String {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("package")
        .trim_end_matches(".git")
        .to_string()
}

/// Clone a remote package and enable it.
pub fn clone_remote(url: &str, name: Option<String>) -> Result<()> {
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
    println!(
        "{}",
        crate::ui::ok(&format!(
            "cloned and enabled package {}",
            crate::ui::header(&name)
        ))
    );
    Ok(())
}

pub fn remove(name: &str) -> Result<()> {
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
    println!("{}", crate::ui::ok(&format!("removed package {name}")));
    Ok(())
}

pub fn list() -> Result<()> {
    let prefs = Prefs::load()?;
    let all = package::list_all()?;
    if all.is_empty() {
        println!("no packages");
        return Ok(());
    }
    for name in all {
        let enabled = prefs.packages.enabled.iter().any(|p| p == &name);
        let is_default = prefs.packages.default == name;
        let badge = if is_default {
            format!(" {}", crate::ui::default_badge())
        } else {
            String::new()
        };
        let dot = if enabled {
            crate::ui::state(crate::ui::dot(), true)
        } else {
            crate::ui::dim(crate::ui::dot())
        };
        let label = if enabled {
            crate::ui::header(&name)
        } else {
            crate::ui::dim(&name)
        };
        println!("{dot} {label}{badge}");
    }
    Ok(())
}

pub fn sync() -> Result<()> {
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

pub fn push(name: &str) -> Result<()> {
    let dir = paths::package_dir(name)?;
    if !dir.join(".git").exists() {
        bail!("package {name} is not a git repo");
    }
    git::commit_and_push(&dir, "duh: update package")?;
    println!("pushed {name}");
    Ok(())
}

pub fn set_enabled(name: &str, enabled: bool) -> Result<()> {
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
pub fn warn_if_ships_functions(name: &str) -> Result<()> {
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
