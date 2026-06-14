//! `duh ls [alias|export|fn] [--package <name>]`

use crate::config::package::{self, Package};
use crate::config::paths;
use crate::config::prefs::Prefs;
use anyhow::{bail, Result};
use clap::ValueEnum;

#[derive(Clone, Copy, ValueEnum)]
pub enum Kind {
    Alias,
    Export,
    Fn,
}

pub fn run(kind: Option<Kind>, package: Option<String>) -> Result<()> {
    let prefs = Prefs::load()?;

    // Resolve which packages to show.
    let packages: Vec<String> = match package {
        Some(name) => {
            if !paths::package_dir(&name)?.exists() {
                bail!("no package {name:?}");
            }
            vec![name]
        }
        None => {
            let enabled = prefs.enabled_existing()?;
            if enabled.is_empty() {
                package::list_all()?
            } else {
                enabled
            }
        }
    };

    let show_alias = matches!(kind, None | Some(Kind::Alias));
    let show_export = matches!(kind, None | Some(Kind::Export));
    let show_fn = matches!(kind, None | Some(Kind::Fn));

    for name in &packages {
        let pkg = Package::load(name)?;
        let funcs = Package::function_files(name)?;
        if pkg.aliases.is_empty() && pkg.exports.is_empty() && funcs.is_empty() {
            continue;
        }
        let path = paths::package_dir(name)?;
        let active = if prefs.packages.default == *name {
            " (default)"
        } else {
            ""
        };
        println!("[{name}]{active}  {}", path.display());

        if show_alias {
            for (k, v) in &pkg.aliases {
                let tag = ssh_tag(pkg.ssh.alias_ok(k));
                println!("  alias  {k} = {v}{tag}");
            }
        }
        if show_export {
            for (k, v) in &pkg.exports {
                let tag = ssh_tag(pkg.ssh.export_ok(k));
                println!("  export {k} = {v}{tag}");
            }
        }
        if show_fn {
            for f in &funcs {
                if let Some(stem) = f.file_stem().and_then(|s| s.to_str()) {
                    match package::function_doc(f) {
                        Some(doc) => println!("  fn     {stem} — {doc}"),
                        None => println!("  fn     {stem}"),
                    }
                }
            }
        }
    }
    Ok(())
}

fn ssh_tag(ok: bool) -> &'static str {
    if ok {
        "  [ssh-safe]"
    } else {
        ""
    }
}
