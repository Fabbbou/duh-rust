//! `duh get [resource] [name] [-p pkg] [--json]`
//!
//! Bare `get` lists everything in the enabled packages. A `resource` filters to
//! one kind; `get pkg` lists packages. Supplying a `name` shows that single item
//! (delegated to `describe`). Machine output (`--json`) is never colored.

use super::resource::Resource;
use crate::config::conflicts;
use crate::config::funcs;
use crate::config::gitcfg;
use crate::config::package::{self, Package};
use crate::config::paths;
use crate::config::prefs::Prefs;
use crate::ui;
use anyhow::{bail, Result};

pub fn run(
    resource: Option<Resource>,
    name: Option<String>,
    package: Option<String>,
    json: bool,
) -> Result<()> {
    // `get <resource> <name>` → show a single item (same as describe).
    if let Some(name) = name {
        let resource = resource.expect("clap guarantees a resource precedes a name");
        return super::describe::run(resource, name, package, json);
    }

    // `get pkg` → list packages.
    if matches!(resource, Some(Resource::Pkg)) {
        return if json {
            pkg_list_json()
        } else {
            super::pkgops::list()
        };
    }

    let prefs = Prefs::load()?;
    let packages = resolve_packages(&prefs, package)?;

    let show_alias = matches!(resource, None | Some(Resource::Alias));
    let show_export = matches!(resource, None | Some(Resource::Export));
    let show_fn = matches!(resource, None | Some(Resource::Fn));
    let show_git = matches!(resource, None | Some(Resource::Gitalias));

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
    render_packages(
        &prefs,
        &packages,
        show_alias,
        show_export,
        show_fn,
        show_git,
    )
}

/// Render the tree view for the given packages. Shared with `describe pkg`.
pub fn render_packages(
    prefs: &Prefs,
    packages: &[String],
    show_alias: bool,
    show_export: bool,
    show_fn: bool,
    show_git: bool,
) -> Result<()> {
    // Cross-package shadowing (last-enabled wins) → mark losing entries.
    let winners = conflicts::winners(&prefs.enabled_existing()?)?;
    let shadow =
        |map: &std::collections::BTreeMap<String, String>, key: &str, pkg: &str| -> String {
            match map.get(key) {
                Some(w) if w != pkg => format!("  {}", ui::dim(&format!("(shadowed by {w})"))),
                _ => String::new(),
            }
        };

    for name in packages {
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

        // Subtle stat line of the non-empty groups.
        let stats: Vec<String> = [
            (n_alias, "alias", "aliases"),
            (n_export, "export", "exports"),
            (n_git, "git alias", "git aliases"),
            (n_files, "script", "scripts"),
        ]
        .iter()
        .filter(|(n, _, _)| *n > 0)
        .map(|(n, one, many)| format!("{n} {}", if *n == 1 { one } else { many }))
        .collect();
        if !stats.is_empty() {
            println!("  {}", ui::dim(&stats.join(" · ")));
        }

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
                    "  {} {} {:<name_w$}  {} {}{}{}",
                    connector(row, total_rows),
                    ui::lbl_alias(),
                    k,
                    ui::arrow(),
                    v,
                    tag,
                    shadow(&winners.aliases, k, name)
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
                    "  {} {} {:<name_w$}  {} {}{}{}",
                    connector(row, total_rows),
                    ui::lbl_export(),
                    k,
                    ui::arrow(),
                    v,
                    tag,
                    shadow(&winners.exports, k, name)
                );
            }
        }
        for (k, v) in &git {
            row += 1;
            println!(
                "  {} {} {:<name_w$}  {} {}",
                connector(row, total_rows),
                ui::lbl_git(),
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
                    ui::lbl_script(),
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
                            ui::lbl_fn(),
                            ui::fn_name(&d.name),
                            ui::dim(s)
                        ),
                        None => println!(
                            "  {}  {} {} {}",
                            cont,
                            fconn,
                            ui::lbl_fn(),
                            ui::fn_name(&d.name)
                        ),
                    }
                }
            }
        }
        println!(); // blank line between packages
    }
    Ok(())
}

/// Tree connector: `└─` for the last row, `├─` otherwise.
pub fn connector(idx: usize, total: usize) -> &'static str {
    if idx >= total {
        ui::ell()
    } else {
        ui::tee()
    }
}

pub fn resolve_packages(prefs: &Prefs, package: Option<String>) -> Result<Vec<String>> {
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

// --- JSON output (never colored) -------------------------------------------

fn pkg_list_json() -> Result<()> {
    let prefs = Prefs::load()?;
    let out: Vec<_> = package::list_all()?
        .into_iter()
        .map(|name| {
            serde_json::json!({
                "name": name,
                "enabled": prefs.packages.enabled.iter().any(|p| p == &name),
                "default": prefs.packages.default == name,
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "packages": out }))?
    );
    Ok(())
}

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
