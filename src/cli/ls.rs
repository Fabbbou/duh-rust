//! `duh ls [alias|export|fn] [--package <name>] [--fn <name>]`

use crate::config::funcs;
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

pub fn run(kind: Option<Kind>, package: Option<String>, func: Option<String>) -> Result<()> {
    let prefs = Prefs::load()?;
    let packages = resolve_packages(&prefs, package)?;

    // `--fn <name>`: full doc for one function, wherever it lives.
    if let Some(name) = func {
        return show_function(&packages, &name);
    }

    let show_alias = matches!(kind, None | Some(Kind::Alias));
    let show_export = matches!(kind, None | Some(Kind::Export));
    let show_fn = matches!(kind, None | Some(Kind::Fn));

    for name in &packages {
        let pkg = Package::load(name)?;
        let files = Package::function_files(name)?;
        if pkg.aliases.is_empty() && pkg.exports.is_empty() && files.is_empty() {
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
                println!("  alias  {k} = {v}{}", ssh_tag(pkg.ssh.alias_ok(k)));
            }
        }
        if show_export {
            for (k, v) in &pkg.exports {
                println!("  export {k} = {v}{}", ssh_tag(pkg.ssh.export_ok(k)));
            }
        }
        if show_fn && !files.is_empty() {
            println!("  functions:");
            for f in &files {
                let script = f.file_name().and_then(|s| s.to_str()).unwrap_or("?");
                println!("    {script}");
                let defs = funcs::parse_functions(f);
                if defs.is_empty() {
                    println!("      (no functions found)");
                }
                for d in defs {
                    match d.summary() {
                        Some(s) => println!("      {} — {s}", d.name),
                        None => println!("      {}", d.name),
                    }
                }
            }
        }
    }
    Ok(())
}

/// Resolve which packages to list: a single named one, else enabled, else all.
fn resolve_packages(prefs: &Prefs, package: Option<String>) -> Result<Vec<String>> {
    match package {
        Some(name) => {
            if !paths::package_dir(&name)?.exists() {
                bail!("no package {name:?}");
            }
            Ok(vec![name])
        }
        None => {
            let enabled = prefs.enabled_existing()?;
            if enabled.is_empty() {
                package::list_all()
            } else {
                Ok(enabled)
            }
        }
    }
}

/// Print the full doc block for a named function across the given packages.
fn show_function(packages: &[String], name: &str) -> Result<()> {
    let mut found = false;
    for pkg in packages {
        for file in Package::function_files(pkg)? {
            for d in funcs::parse_functions(&file) {
                if d.name == name {
                    found = true;
                    let script = file.file_name().and_then(|s| s.to_str()).unwrap_or("?");
                    println!("{} [{pkg}] {}", d.name, file.display());
                    println!("  script: {script}");
                    if d.doc.is_empty() {
                        println!("  (no documentation)");
                    } else {
                        for line in &d.doc {
                            println!("  {line}");
                        }
                    }
                    println!();
                }
            }
        }
    }
    if !found {
        bail!(
            "no function named {name:?} in {} package(s)",
            packages.len()
        );
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
