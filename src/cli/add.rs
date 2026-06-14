//! `duh add alias|export|fn`

use crate::config::package::Package;
use crate::config::paths;
use crate::config::prefs::Prefs;
use crate::inject::escape;
use anyhow::{Context, Result};
use clap::Subcommand;
use std::fs;
use std::process::Command;

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
            println!("added alias {name}{tag} → package \"{target}\"");
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
            println!("added export {name}{tag} → package \"{target}\"");
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
            open_editor(&file)?;
            // Warn-only lint after editing.
            for w in crate::config::package::function_lint(&file) {
                eprintln!("warning: {w}");
            }
            println!("saved function {name} → package \"{target}\"");
        }
    }
    Ok(())
}

fn open_editor(path: &std::path::Path) -> Result<()> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(&editor)
        .arg(path)
        .status()
        .with_context(|| format!("launching editor {editor}"))?;
    if !status.success() {
        anyhow::bail!("editor {editor} exited with failure");
    }
    Ok(())
}
