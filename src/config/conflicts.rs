//! Cross-package conflict detection: when several enabled packages define the
//! same alias/export name, the last-enabled one wins at injection. Shared by
//! `duh doctor` (report) and `duh ls` (shadowed markers).

use crate::config::package::Package;
use crate::config::prefs::Prefs;
use anyhow::Result;
use std::collections::BTreeMap;

/// A name defined by more than one enabled package.
pub struct Conflict {
    pub kind: &'static str, // "alias" | "export"
    pub name: String,
    pub packages: Vec<String>, // in enabled order
    pub winner: String,        // last enabled = wins
}

/// Winner package for each conflicting name (for `ls` shadow markers).
#[derive(Default)]
pub struct Winners {
    pub aliases: BTreeMap<String, String>,
    pub exports: BTreeMap<String, String>,
}

/// Build per-name → winning-package maps over the enabled packages (in order).
/// Only names defined by 2+ packages are included.
pub fn winners(enabled: &[String]) -> Result<Winners> {
    let mut alias_seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut export_seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in enabled {
        let pkg = Package::load(name)?;
        for k in pkg.aliases.keys() {
            alias_seen.entry(k.clone()).or_default().push(name.clone());
        }
        for k in pkg.exports.keys() {
            export_seen.entry(k.clone()).or_default().push(name.clone());
        }
    }
    let mut w = Winners::default();
    for (k, pkgs) in alias_seen {
        if pkgs.len() > 1 {
            w.aliases.insert(k, pkgs.last().unwrap().clone());
        }
    }
    for (k, pkgs) in export_seen {
        if pkgs.len() > 1 {
            w.exports.insert(k, pkgs.last().unwrap().clone());
        }
    }
    Ok(w)
}

/// All conflicts across the currently enabled packages.
pub fn find() -> Result<Vec<Conflict>> {
    let prefs = Prefs::load()?;
    let enabled = prefs.enabled_existing()?;

    let mut alias_seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut export_seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in &enabled {
        let pkg = Package::load(name)?;
        for k in pkg.aliases.keys() {
            alias_seen.entry(k.clone()).or_default().push(name.clone());
        }
        for k in pkg.exports.keys() {
            export_seen.entry(k.clone()).or_default().push(name.clone());
        }
    }

    let mut out = Vec::new();
    for (kind, seen) in [("alias", alias_seen), ("export", export_seen)] {
        for (name, packages) in seen {
            if packages.len() > 1 {
                let winner = packages.last().unwrap().clone();
                out.push(Conflict {
                    kind,
                    name,
                    packages,
                    winner,
                });
            }
        }
    }
    Ok(out)
}
