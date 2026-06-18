//! `duh create <resource> <name> [value] [--ssh-safe] [-p pkg] [--remote URL]`

use super::pkgops;
use super::resource::Resource;
use crate::config::package::Package;
use crate::config::paths;
use crate::config::prefs::Prefs;
use crate::inject::escape;
use crate::ui;
use anyhow::{bail, Result};
use std::fs;

pub fn run(
    resource: Resource,
    name: String,
    value: Option<String>,
    ssh_safe: bool,
    package: Option<String>,
    remote: Option<String>,
) -> Result<()> {
    match resource {
        Resource::Pkg => {
            if value.is_some() {
                bail!("`create pkg` takes no value argument");
            }
            match remote {
                Some(url) => pkgops::clone_remote(&url, Some(name)),
                None => pkgops::create_empty(&name),
            }
        }
        Resource::Fn => {
            if value.is_some() {
                bail!("`create fn` takes no value argument (the body is edited in $EDITOR)");
            }
            create_fn(&target(package)?, &name)
        }
        Resource::Alias | Resource::Export | Resource::Gitalias => {
            let value = value
                .ok_or_else(|| anyhow::anyhow!("`create {}` needs a value", resource.label()))?;
            create_entry(resource, &target(package)?, &name, &value, ssh_safe)
        }
    }
}

/// The package a create/delete acts on: `-p` override or the default package.
fn target(package: Option<String>) -> Result<String> {
    match package {
        Some(p) => {
            if !paths::package_dir(&p)?.exists() {
                bail!("no package {p:?}");
            }
            Ok(p)
        }
        None => Ok(Prefs::load()?.packages.default),
    }
}

fn create_entry(
    resource: Resource,
    target: &str,
    name: &str,
    value: &str,
    ssh_safe: bool,
) -> Result<()> {
    if resource == Resource::Gitalias {
        crate::config::gitcfg::set_alias(target, name, value)?;
        println!(
            "{}",
            ui::ok(&format!(
                "created git alias {name} → package {} (run `duh inject` to wire it into ~/.gitconfig)",
                ui::header(target)
            ))
        );
        return Ok(());
    }

    escape::require_valid_name(resource.label(), name)?;
    let mut pkg = Package::load(target)?;
    match resource {
        Resource::Alias => {
            pkg.aliases.insert(name.to_string(), value.to_string());
            if ssh_safe {
                pkg.ssh.flag_alias(name);
            }
        }
        Resource::Export => {
            pkg.exports.insert(name.to_string(), value.to_string());
            if ssh_safe {
                pkg.ssh.flag_export(name);
            }
        }
        _ => unreachable!(),
    }
    pkg.save(target)?;
    let tag = if ssh_safe { " [ssh-safe]" } else { "" };
    println!(
        "{}",
        ui::ok(&format!(
            "created {} {name}{tag} → package {}",
            resource.label(),
            ui::header(target)
        ))
    );
    Ok(())
}

fn create_fn(target: &str, name: &str) -> Result<()> {
    escape::require_valid_name("function", name)?;
    let dir = paths::package_functions_dir(target)?;
    fs::create_dir_all(&dir)?;
    let file = dir.join(format!("{name}.sh"));
    if !file.exists() {
        fs::write(
            &file,
            format!("# function: {name}\n{name}() {{\n  : # TODO\n}}\n"),
        )?;
    }
    super::editor::open_in_editor(&file)?;
    // Warn-only lint after editing.
    for w in crate::config::package::function_lint(&file) {
        eprintln!("{}", ui::warn(&w));
    }
    println!(
        "{}",
        ui::ok(&format!(
            "saved function {name} → package {}",
            ui::header(target)
        ))
    );
    Ok(())
}
