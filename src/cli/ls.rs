//! `duh ls [alias|export|fn] [--package <name>] [--fn <name>] [--json]`

use crate::config::funcs;
use crate::config::gitcfg;
use crate::config::package::{self, Package};
use crate::config::paths;
use crate::config::prefs::Prefs;
use crate::ui;
use anyhow::{bail, Result};
use clap::ValueEnum;

#[derive(Clone, Copy, ValueEnum)]
pub enum Kind {
    Alias,
    Export,
    Fn,
    Git,
}

pub fn run(
    kind: Option<Kind>,
    package: Option<String>,
    func: Option<String>,
    json: bool,
) -> Result<()> {
    let prefs = Prefs::load()?;
    let packages = resolve_packages(&prefs, package)?;

    if let Some(name) = func {
        return if json {
            show_function_json(&packages, &name)
        } else {
            show_function(&packages, &name)
        };
    }

    let show_alias = matches!(kind, None | Some(Kind::Alias));
    let show_export = matches!(kind, None | Some(Kind::Export));
    let show_fn = matches!(kind, None | Some(Kind::Fn));
    let show_git = matches!(kind, None | Some(Kind::Git));

    if json {
        return list_json(
            &prefs,
            &packages,
            show_alias,
            show_export,
            show_fn,
            show_git,
        );
    }

    for name in &packages {
        let pkg = Package::load(name)?;
        let files = Package::function_files(name)?;
        let git = if show_git {
            gitcfg::aliases(name)?
        } else {
            Vec::new()
        };
        if pkg.aliases.is_empty() && pkg.exports.is_empty() && files.is_empty() && git.is_empty() {
            continue;
        }

        // Header: ● name (default)
        let is_default = prefs.packages.default == *name;
        let badge = if is_default {
            format!(" {}", ui::default_badge())
        } else {
            String::new()
        };
        println!("{} {}{}", ui::dot(), ui::header(name), badge);
        println!(
            "  {}",
            ui::dim(&paths::package_dir(name)?.display().to_string())
        );

        // Count top-level rows so the last gets `└─`.
        let n_alias = if show_alias { pkg.aliases.len() } else { 0 };
        let n_export = if show_export { pkg.exports.len() } else { 0 };
        let n_git = git.len();
        let n_files = if show_fn { files.len() } else { 0 };
        let total_rows = n_alias + n_export + n_git + n_files;
        let mut row = 0usize;

        // Align the value column across alias/export/git names.
        let name_w = pkg
            .aliases
            .keys()
            .chain(pkg.exports.keys())
            .map(|k| k.len())
            .chain(git.iter().map(|(k, _)| k.len()))
            .max()
            .unwrap_or(0);

        if show_alias {
            for (k, v) in &pkg.aliases {
                row += 1;
                let tag = if pkg.ssh.alias_ok(k) {
                    format!("  {}", ui::badge_ssh())
                } else {
                    String::new()
                };
                println!(
                    "  {} {} {:<name_w$}  {} {}{}",
                    connector(row, total_rows),
                    ui::dim("alias "),
                    k,
                    ui::arrow(),
                    v,
                    tag
                );
            }
        }
        if show_export {
            for (k, v) in &pkg.exports {
                row += 1;
                let tag = if pkg.ssh.export_ok(k) {
                    format!("  {}", ui::badge_ssh())
                } else {
                    String::new()
                };
                println!(
                    "  {} {} {:<name_w$}  {} {}{}",
                    connector(row, total_rows),
                    ui::dim("export"),
                    k,
                    ui::arrow(),
                    v,
                    tag
                );
            }
        }
        for (k, v) in &git {
            row += 1;
            println!(
                "  {} {} {:<name_w$}  {} {}",
                connector(row, total_rows),
                ui::dim("git   "),
                k,
                ui::arrow(),
                v
            );
        }
        if show_fn {
            for f in &files {
                row += 1;
                let last_file = row == total_rows;
                let script = f.file_name().and_then(|s| s.to_str()).unwrap_or("?");
                println!(
                    "  {} {} {}",
                    connector(row, total_rows),
                    ui::dim("script"),
                    ui::script_name(script)
                );

                // Continue the parent's vertical bar unless it was the last row.
                let cont = if last_file { " " } else { ui::pipe() };
                let defs = funcs::parse_functions(f);
                if defs.is_empty() {
                    println!("  {}    {}", cont, ui::dim("(no functions found)"));
                }
                let fn_w = defs.iter().map(|d| d.name.len()).max().unwrap_or(0);
                for (i, d) in defs.iter().enumerate() {
                    let fconn = connector(i + 1, defs.len());
                    match d.summary() {
                        Some(s) => println!(
                            "  {}  {} {} {:<fn_w$}  {}",
                            cont,
                            fconn,
                            ui::dim("fn"),
                            ui::fn_name(&d.name),
                            ui::dim(s)
                        ),
                        None => println!(
                            "  {}  {} {} {}",
                            cont,
                            fconn,
                            ui::dim("fn"),
                            ui::fn_name(&d.name)
                        ),
                    }
                }
            }
        }
    }
    Ok(())
}

/// Tree connector: `└─` for the last row, `├─` otherwise.
fn connector(idx: usize, total: usize) -> &'static str {
    if idx >= total {
        ui::ell()
    } else {
        ui::tee()
    }
}

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

fn show_function(packages: &[String], name: &str) -> Result<()> {
    let mut found = false;
    for pkg in packages {
        for file in Package::function_files(pkg)? {
            for d in funcs::parse_functions(&file) {
                if d.name == name {
                    found = true;
                    println!(
                        "{} {}  {}",
                        ui::fn_name(&d.name),
                        ui::dim(&format!("[{pkg}]")),
                        ui::dim(&file.display().to_string())
                    );
                    if d.doc.is_empty() {
                        println!("  {}", ui::dim("(no documentation)"));
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

// --- JSON output (never colored) -------------------------------------------

fn list_json(
    prefs: &Prefs,
    packages: &[String],
    show_alias: bool,
    show_export: bool,
    show_fn: bool,
    show_git: bool,
) -> Result<()> {
    let mut out = Vec::new();
    for name in packages {
        let pkg = Package::load(name)?;
        let aliases: Vec<_> = if show_alias {
            pkg.aliases
                .iter()
                .map(|(k, v)| {
                    serde_json::json!({"name": k, "value": v, "ssh_safe": pkg.ssh.alias_ok(k)})
                })
                .collect()
        } else {
            Vec::new()
        };
        let exports: Vec<_> = if show_export {
            pkg.exports
                .iter()
                .map(|(k, v)| {
                    serde_json::json!({"name": k, "value": v, "ssh_safe": pkg.ssh.export_ok(k)})
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut functions = Vec::new();
        if show_fn {
            for f in Package::function_files(name)? {
                let script = f.file_name().and_then(|s| s.to_str()).unwrap_or("?");
                for d in funcs::parse_functions(&f) {
                    functions.push(serde_json::json!({
                        "script": script, "name": d.name, "doc": d.doc.join("\n")
                    }));
                }
            }
        }
        let git: Vec<_> = if show_git {
            gitcfg::aliases(name)?
                .into_iter()
                .map(|(k, v)| serde_json::json!({"name": k, "value": v}))
                .collect()
        } else {
            Vec::new()
        };
        let gitconfig = if gitcfg::has_gitconfig(name) {
            serde_json::Value::String(paths::package_gitconfig(name)?.display().to_string())
        } else {
            serde_json::Value::Null
        };
        out.push(serde_json::json!({
            "name": name,
            "default": prefs.packages.default == *name,
            "path": paths::package_dir(name)?.display().to_string(),
            "aliases": aliases,
            "exports": exports,
            "functions": functions,
            "git": git,
            "gitconfig": gitconfig,
        }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "packages": out }))?
    );
    Ok(())
}

fn show_function_json(packages: &[String], name: &str) -> Result<()> {
    let mut matches = Vec::new();
    for pkg in packages {
        for file in Package::function_files(pkg)? {
            let script = file.file_name().and_then(|s| s.to_str()).unwrap_or("?");
            for d in funcs::parse_functions(&file) {
                if d.name == name {
                    matches.push(serde_json::json!({
                        "name": d.name, "package": pkg, "script": script,
                        "path": file.display().to_string(), "doc": d.doc.join("\n"),
                    }));
                }
            }
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "functions": matches }))?
    );
    Ok(())
}
