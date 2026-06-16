//! `duh add alias|export|fn`

use crate::config::package::Package;
use crate::config::paths;
use crate::config::prefs::Prefs;
use crate::inject::escape;
use anyhow::Result;
use clap::Subcommand;
use std::fs;

#[derive(Subcommand)]
pub enum AddCmd {
    /// Add or update a shell alias
    Alias {
        name: String,
        value: String,
        /// Also flag this alias as safe to inject over SSH
        #[arg(long)]
        ssh_safe: bool,
    },
    /// Add or update an environment export
    Export {
        name: String,
        value: String,
        /// Also flag this export as safe to inject over SSH
        #[arg(long)]
        ssh_safe: bool,
    },
    /// Create a function file and open it in $EDITOR
    Fn { name: String },
    /// Manage the package's git config (e.g. `duh add git alias co checkout`)
    Git {
        #[command(subcommand)]
        what: GitAddCmd,
    },
}

#[derive(Subcommand)]
pub enum GitAddCmd {
    /// Add or update a git alias in the package's gitconfig
    Alias { name: String, value: String },
}

pub fn run(cmd: AddCmd) -> Result<()> {
    let target = Prefs::load()?.packages.default;
    match cmd {
        AddCmd::Alias {
            name,
            value,
            ssh_safe,
        } => {
            escape::require_valid_name("alias", &name)?;
            let mut pkg = Package::load(&target)?;
            pkg.aliases.insert(name.clone(), value);
            if ssh_safe {
                pkg.ssh.flag_alias(&name);
            }
            pkg.save(&target)?;
            let tag = if ssh_safe { " [ssh-safe]" } else { "" };
            println!(
                "{}",
                crate::ui::ok(&format!(
                    "added alias {name}{tag} → package {}",
                    crate::ui::header(&target)
                ))
            );
        }
        AddCmd::Export {
            name,
            value,
            ssh_safe,
        } => {
            escape::require_valid_name("export", &name)?;
            let mut pkg = Package::load(&target)?;
            pkg.exports.insert(name.clone(), value);
            if ssh_safe {
                pkg.ssh.flag_export(&name);
            }
            pkg.save(&target)?;
            let tag = if ssh_safe { " [ssh-safe]" } else { "" };
            println!(
                "{}",
                crate::ui::ok(&format!(
                    "added export {name}{tag} → package {}",
                    crate::ui::header(&target)
                ))
            );
        }
        AddCmd::Fn { name } => {
            escape::require_valid_name("function", &name)?;
            let dir = paths::package_functions_dir(&target)?;
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
                eprintln!("{}", crate::ui::warn(&w));
            }
            println!(
                "{}",
                crate::ui::ok(&format!(
                    "saved function {name} → package {}",
                    crate::ui::header(&target)
                ))
            );
        }
        AddCmd::Git {
            what: GitAddCmd::Alias { name, value },
        } => {
            crate::config::gitcfg::set_alias(&target, &name, &value)?;
            println!(
                "{}",
                crate::ui::ok(&format!(
                    "added git alias {name} → package {} (run `duh inject` to wire it into ~/.gitconfig)",
                    crate::ui::header(&target)
                ))
            );
        }
    }
    Ok(())
}
