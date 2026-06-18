//! `duh describe <resource> <name> [-p pkg] [--json]` — detailed view of one item.

use super::get;
use super::resource::Resource;
use crate::config::funcs;
use crate::config::gitcfg;
use crate::config::package::Package;
use crate::config::paths;
use crate::config::prefs::Prefs;
use crate::ui;
use anyhow::{bail, Result};

pub fn run(resource: Resource, name: String, package: Option<String>, json: bool) -> Result<()> {
    match resource {
        Resource::Fn => describe_fn(package, &name, json),
        Resource::Pkg => describe_pkg(&name, json),
        Resource::Alias => describe_entry(resource, package, &name, json),
        Resource::Export => describe_entry(resource, package, &name, json),
        Resource::Gitalias => describe_gitalias(package, &name, json),
    }
}

fn resolve(package: Option<String>) -> Result<Vec<String>> {
    let prefs = Prefs::load()?;
    get::resolve_packages(&prefs, package)
}

// --- functions -------------------------------------------------------------

fn describe_fn(package: Option<String>, name: &str, json: bool) -> Result<()> {
    let packages = resolve(package)?;
    if json {
        let mut matches = Vec::new();
        for pkg in &packages {
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
        return Ok(());
    }

    let mut found = false;
    for pkg in &packages {
        for file in Package::function_files(pkg)? {
            for d in funcs::parse_functions(&file) {
                if d.name != name {
                    continue;
                }
                found = true;
                let script = file.file_name().and_then(|s| s.to_str()).unwrap_or("?");
                println!("{}", ui::fn_name(&d.name));
                println!("  {}  {}", ui::field("package"), pkg);
                println!("  {}   {}", ui::field("script"), ui::script_name(script));
                println!(
                    "  {}     {}",
                    ui::field("path"),
                    ui::dim(&file.display().to_string())
                );
                println!("  {}", ui::field("doc"));
                if d.doc.is_empty() {
                    println!("    {}", ui::dim("(no documentation)"));
                } else {
                    for line in &d.doc {
                        println!("    {line}");
                    }
                }
                println!();
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

// --- packages --------------------------------------------------------------

fn describe_pkg(name: &str, json: bool) -> Result<()> {
    if !paths::package_dir(name)?.exists() {
        bail!("no package {name:?}");
    }
    let prefs = Prefs::load()?;
    let enabled = prefs.packages.enabled.iter().any(|p| p == name);
    let is_default = prefs.packages.default == name;

    if json {
        let pkg = Package::load(name)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "name": name,
                "path": paths::package_dir(name)?.display().to_string(),
                "enabled": enabled,
                "default": is_default,
                "aliases": pkg.aliases.len(),
                "exports": pkg.exports.len(),
                "functions": Package::function_files(name)?.len(),
                "gitconfig": gitcfg::has_gitconfig(name),
            }))?
        );
        return Ok(());
    }

    let state = if enabled {
        ui::state("enabled", true)
    } else {
        ui::state("disabled", false)
    };
    let badge = if is_default {
        format!(" {}", ui::default_badge())
    } else {
        String::new()
    };
    println!("{} {}{}  {}", ui::dot(), ui::header(name), badge, state);
    // Reuse the entry tree for the single package (all kinds).
    get::render_packages(&prefs, &[name.to_string()], true, true, true, true)
}

// --- single alias / export -------------------------------------------------

fn describe_entry(
    resource: Resource,
    package: Option<String>,
    name: &str,
    json: bool,
) -> Result<()> {
    for pkg_name in resolve(package)? {
        let pkg = Package::load(&pkg_name)?;
        let (value, ssh_safe) = match resource {
            Resource::Alias => (pkg.aliases.get(name), pkg.ssh.alias_ok(name)),
            Resource::Export => (pkg.exports.get(name), pkg.ssh.export_ok(name)),
            _ => unreachable!(),
        };
        if let Some(value) = value {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "name": name, "value": value, "package": pkg_name,
                        "kind": resource.label(), "ssh_safe": ssh_safe,
                    }))?
                );
            } else {
                let tag = if ssh_safe {
                    format!("  {}", ui::badge_ssh())
                } else {
                    String::new()
                };
                println!("{}", ui::fn_name(name));
                println!("  {}  {}", ui::field("package"), pkg_name);
                println!(
                    "  {}     {} {}{}",
                    ui::field("value"),
                    ui::arrow(),
                    value,
                    tag
                );
            }
            return Ok(());
        }
    }
    bail!("no {} named {name:?}", resource.label());
}

fn describe_gitalias(package: Option<String>, name: &str, json: bool) -> Result<()> {
    for pkg_name in resolve(package)? {
        for (k, v) in gitcfg::aliases(&pkg_name)? {
            if k == name {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "name": name, "value": v, "package": pkg_name, "kind": "git alias",
                        }))?
                    );
                } else {
                    println!("{}", ui::fn_name(name));
                    println!("  {}  {}", ui::field("package"), pkg_name);
                    println!("  {}     {} {}", ui::field("value"), ui::arrow(), v);
                }
                return Ok(());
            }
        }
    }
    bail!("no git alias named {name:?}");
}
