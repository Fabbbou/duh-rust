//! `duh rm alias|export|fn`

use crate::config::package::Package;
use crate::config::paths;
use crate::config::prefs::Prefs;
use anyhow::{bail, Result};
use clap::Subcommand;
use std::fs;

#[derive(Subcommand)]
pub enum RmCmd {
    /// Remove a shell alias
    Alias { name: String },
    /// Remove an environment export
    Export { name: String },
    /// Remove a function file
    Fn { name: String },
}

pub fn run(cmd: RmCmd) -> Result<()> {
    let target = Prefs::load()?.packages.default;
    match cmd {
        RmCmd::Alias { name } => {
            let mut pkg = Package::load(&target)?;
            if pkg.aliases.remove(&name).is_none() {
                bail!("no alias {name:?} in package {target}");
            }
            pkg.unflag_ssh(&name);
            pkg.save(&target)?;
            println!(
                "{}",
                crate::ui::ok(&format!(
                    "removed alias {name} from package {}",
                    crate::ui::header(&target)
                ))
            );
        }
        RmCmd::Export { name } => {
            let mut pkg = Package::load(&target)?;
            if pkg.exports.remove(&name).is_none() {
                bail!("no export {name:?} in package {target}");
            }
            pkg.unflag_ssh(&name);
            pkg.save(&target)?;
            println!(
                "{}",
                crate::ui::ok(&format!(
                    "removed export {name} from package {}",
                    crate::ui::header(&target)
                ))
            );
        }
        RmCmd::Fn { name } => {
            let file = paths::package_functions_dir(&target)?.join(format!("{name}.sh"));
            if !file.exists() {
                bail!("no function {name:?} in package {target}");
            }
            fs::remove_file(&file)?;
            println!(
                "{}",
                crate::ui::ok(&format!(
                    "removed function {name} from package {}",
                    crate::ui::header(&target)
                ))
            );
        }
    }
    Ok(())
}
