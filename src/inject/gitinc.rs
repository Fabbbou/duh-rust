//! Sync per-package `gitconfig` files into the user's `~/.gitconfig` via the
//! `[include]` mechanism — git then transparently reads each package's git
//! aliases/settings. Add-only and idempotent; runs on local `inject` (never SSH).

use crate::config::paths;
use crate::config::prefs::Prefs;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// For each enabled package that ships a `gitconfig`, ensure `~/.gitconfig` has
/// an `[include] path = …` line pointing at it. Returns the paths newly added.
pub fn sync_enabled() -> Result<Vec<PathBuf>> {
    let prefs = Prefs::load()?;
    let mut wanted = Vec::new();
    for name in prefs.enabled_existing()? {
        let gc = paths::package_gitconfig(&name)?;
        if gc.exists() {
            wanted.push(gc);
        }
    }
    sync(&wanted)
}

/// Ensure each path in `wanted` appears under an `[include]` section of the
/// user's gitconfig. Preserves all existing content; only appends missing lines.
fn sync(wanted: &[PathBuf]) -> Result<Vec<PathBuf>> {
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let gc_path = paths::git_config_path()?;
    let existing = fs::read_to_string(&gc_path).unwrap_or_default();

    // Every `path = …` value already present anywhere in the file (dedupe set).
    let present: HashSet<String> = existing
        .lines()
        .filter_map(parse_path_value)
        .map(|s| s.to_string())
        .collect();

    let missing: Vec<&PathBuf> = wanted
        .iter()
        .filter(|p| !present.contains(&p.to_string_lossy().into_owned()))
        .collect();
    if missing.is_empty() {
        return Ok(Vec::new());
    }

    let new_content = insert_includes(&existing, &missing);
    if let Some(parent) = gc_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&gc_path, new_content).with_context(|| format!("writing {}", gc_path.display()))?;
    Ok(missing.into_iter().cloned().collect())
}

/// Value of a `path = X` line inside an include section (trimmed), else None.
fn parse_path_value(line: &str) -> Option<&str> {
    let t = line.trim();
    let rest = t.strip_prefix("path")?;
    let rest = rest.trim_start();
    let val = rest.strip_prefix('=')?;
    Some(val.trim())
}

/// Insert `missing` path lines under an existing `[include]` header, or append a
/// new `[include]` section if none exists.
fn insert_includes(existing: &str, missing: &[&PathBuf]) -> String {
    let lines: Vec<&str> = existing.lines().collect();
    let header = lines.iter().position(|l| l.trim() == "[include]");
    let added: String = missing
        .iter()
        .map(|p| format!("\tpath = {}\n", p.display()))
        .collect();

    match header {
        Some(idx) => {
            // Re-emit with the new path lines right after the [include] header.
            let mut out = String::new();
            for (i, line) in lines.iter().enumerate() {
                out.push_str(line);
                out.push('\n');
                if i == idx {
                    out.push_str(&added);
                }
            }
            out
        }
        None => {
            let mut out = existing.to_string();
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("[include]\n");
            out.push_str(&added);
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_include_section_when_absent() {
        let out = insert_includes("", &[&PathBuf::from("/a/gitconfig")]);
        assert!(out.contains("[include]"));
        assert!(out.contains("path = /a/gitconfig"));
    }

    #[test]
    fn inserts_under_existing_include() {
        let existing = "[include]\n\tpath = /existing\n[user]\n\tname = x\n";
        let out = insert_includes(existing, &[&PathBuf::from("/new")]);
        assert!(out.contains("path = /existing"));
        assert!(out.contains("path = /new"));
        assert!(out.contains("[user]"));
        // new path sits within the include section (before [user]).
        assert!(out.find("path = /new").unwrap() < out.find("[user]").unwrap());
    }

    #[test]
    fn parses_path_values() {
        assert_eq!(parse_path_value("\tpath = /x"), Some("/x"));
        assert_eq!(parse_path_value("  path=/y"), Some("/y"));
        assert_eq!(parse_path_value("name = foo"), None);
    }
}
