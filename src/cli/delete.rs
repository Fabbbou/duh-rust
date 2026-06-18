//! `duh delete <resource> <name> [-p pkg]`

use super::pkgops;
use super::resource::Resource;
use crate::config::package::Package;
use crate::config::paths;
use crate::config::prefs::Prefs;
use crate::ui;
use anyhow::{bail, Result};
use std::fs;

pub fn run(resource: Resource, name: String, package: Option<String>) -> Result<()> {
    if resource == Resource::Pkg {
        return pkgops::remove(&name);
    }
    let target = target(package)?;
    match resource {
        Resource::Alias => {
            let mut pkg = Package::load(&target)?;
            if pkg.aliases.remove(&name).is_none() {
                bail!("no alias {name:?} in package {target}");
            }
            pkg.unflag_ssh(&name);
            pkg.save(&target)?;
            done("alias", &name, &target);
        }
        Resource::Export => {
            let mut pkg = Package::load(&target)?;
            if pkg.exports.remove(&name).is_none() {
                bail!("no export {name:?} in package {target}");
            }
            pkg.unflag_ssh(&name);
            pkg.save(&target)?;
            done("export", &name, &target);
        }
        Resource::Fn => {
            let file = paths::package_functions_dir(&target)?.join(format!("{name}.sh"));
            if !file.exists() {
                bail!("no function {name:?} in package {target}");
            }
            fs::remove_file(&file)?;
            done("function", &name, &target);
        }
        Resource::Gitalias => {
            crate::config::gitcfg::remove_alias(&target, &name)?;
            done("git alias", &name, &target);
        }
        Resource::Pkg => unreachable!(),
    }
    Ok(())
}

fn done(kind: &str, name: &str, target: &str) {
    println!(
        "{}",
        ui::ok(&format!(
            "removed {kind} {name} from package {}",
            ui::header(target)
        ))
    );
}

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
