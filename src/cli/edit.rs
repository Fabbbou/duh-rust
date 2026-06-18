//! `duh edit <resource> [name] [-p pkg]` — open something in `$EDITOR`.
//!
//! Only `fn` and `pkg` are editable in `$EDITOR`. Alias/export/gitalias values
//! are changed with `create` (it upserts), so editing them in an editor makes no
//! sense — we point the user there instead.

use super::resource::Resource;
use crate::config::package::Package;
use crate::config::paths;
use crate::config::prefs::Prefs;
use crate::ui;
use anyhow::{bail, Result};

pub fn run(resource: Resource, name: Option<String>, package: Option<String>) -> Result<()> {
    match resource {
        Resource::Pkg => edit_pkg(name.or(package)),
        Resource::Fn => {
            let name = name.ok_or_else(|| anyhow::anyhow!("`edit fn` needs a function name"))?;
            edit_fn(target(package)?, &name)
        }
        Resource::Alias | Resource::Export | Resource::Gitalias => {
            bail!(
                "`edit` supports only `fn` and `pkg`; change a {} with `duh create {} <name> <value>`",
                resource.label(),
                clap_token(resource),
            )
        }
    }
}

fn clap_token(resource: Resource) -> &'static str {
    match resource {
        Resource::Alias => "alias",
        Resource::Export => "export",
        Resource::Gitalias => "gitalias",
        _ => unreachable!(),
    }
}

fn target(package: Option<String>) -> Result<String> {
    match package {
        Some(p) => Ok(p),
        None => Ok(Prefs::load()?.packages.default),
    }
}

fn edit_pkg(package: Option<String>) -> Result<()> {
    let name = match package {
        Some(p) => p,
        None => Prefs::load()?.packages.default,
    };
    // Validates name; create the db.toml if the package doesn't exist yet.
    let db = paths::package_db(&name)?;
    if !db.exists() {
        Package::default().save(&name)?;
    }
    super::editor::open_in_editor(&db)?;
    println!(
        "{}",
        ui::ok(&format!("edited package {}", ui::header(&name)))
    );
    Ok(())
}

fn edit_fn(target: String, name: &str) -> Result<()> {
    let file = paths::package_functions_dir(&target)?.join(format!("{name}.sh"));
    if !file.exists() {
        bail!("no function {name:?} in package {target} (create it with `duh create fn {name}`)");
    }
    super::editor::open_in_editor(&file)?;
    for w in crate::config::package::function_lint(&file) {
        eprintln!("{}", ui::warn(&w));
    }
    println!(
        "{}",
        ui::ok(&format!(
            "edited function {name} → package {}",
            ui::header(&target)
        ))
    );
    Ok(())
}
