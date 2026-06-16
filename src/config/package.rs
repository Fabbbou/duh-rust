//! Package model: a named TOML bundle of aliases, exports, and function scripts.

use crate::config::paths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
    #[serde(default = "crate::config::default_schema")]
    pub schema: u32,
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
    #[serde(default)]
    pub exports: BTreeMap<String, String>,
    /// Names explicitly flagged safe to inject over SSH (opt-in allowlist).
    #[serde(default, skip_serializing_if = "SshSafe::is_empty")]
    pub ssh: SshSafe,
    #[serde(default)]
    pub metadata: Metadata,
}

impl Default for Package {
    fn default() -> Self {
        Package {
            schema: crate::config::SCHEMA_VERSION,
            aliases: BTreeMap::new(),
            exports: BTreeMap::new(),
            ssh: SshSafe::default(),
            metadata: Metadata::default(),
        }
    }
}

/// Opt-in allowlist of alias/export names that may be shipped to remote hosts.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SshSafe {
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub exports: Vec<String>,
}

impl SshSafe {
    fn is_empty(&self) -> bool {
        self.aliases.is_empty() && self.exports.is_empty()
    }

    pub fn alias_ok(&self, name: &str) -> bool {
        self.aliases.iter().any(|n| n == name)
    }

    pub fn export_ok(&self, name: &str) -> bool {
        self.exports.iter().any(|n| n == name)
    }

    /// Add a name once (idempotent), keeping the list sorted.
    pub fn flag_alias(&mut self, name: &str) {
        if !self.alias_ok(name) {
            self.aliases.push(name.to_string());
            self.aliases.sort();
        }
    }

    pub fn flag_export(&mut self, name: &str) {
        if !self.export_ok(name) {
            self.exports.push(name.to_string());
            self.exports.sort();
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(default)]
    pub url_origin: String,
    #[serde(default)]
    pub name_origin: String,
}

impl Package {
    /// Load a package's `db.toml`, returning an empty package if the file is absent.
    pub fn load(name: &str) -> Result<Package> {
        let path = paths::package_db(name)?;
        if !path.exists() {
            return Ok(Package::default());
        }
        let raw =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let pkg: Package =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        crate::config::warn_if_newer(pkg.schema, &format!("package {name}"));
        Ok(pkg)
    }

    /// Persist this package's `db.toml`, creating directories as needed.
    pub fn save(&self, name: &str) -> Result<()> {
        let dir = paths::package_dir(name)?;
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = paths::package_db(name)?;
        let raw = toml::to_string_pretty(self).context("serializing package")?;
        fs::write(&path, raw).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Remove a name from the SSH allowlists if present (called when an alias or
    /// export is deleted, so flags never dangle).
    pub fn unflag_ssh(&mut self, name: &str) {
        self.ssh.aliases.retain(|n| n != name);
        self.ssh.exports.retain(|n| n != name);
    }

    /// Paths to all `*.sh` function files in this package, sorted.
    pub fn function_files(name: &str) -> Result<Vec<PathBuf>> {
        let dir = paths::package_functions_dir(name)?;
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .with_context(|| format!("reading {}", dir.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().map(|x| x == "sh").unwrap_or(false))
            .collect();
        files.sort();
        Ok(files)
    }
}

/// Lightweight, warn-only lint: a function file should contain only function
/// definitions (and comments/blanks). Returns human-readable warnings for any
/// statement found at brace-depth 0 that isn't a comment, blank, or function
/// header. Heuristic (naive brace counting) — good enough to catch a stray
/// `rm -rf` pasted into a functions script; never blocks injection.
pub fn function_lint(path: &std::path::Path) -> Vec<String> {
    let Ok(body) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut warnings = Vec::new();
    let mut depth: i32 = 0;
    for (i, raw) in body.lines().enumerate() {
        let line = raw.trim();
        let is_header = crate::config::funcs::name_from_header(line).is_some();
        if depth == 0 && !line.is_empty() && !line.starts_with('#') && !is_header {
            warnings.push(format!(
                "{}:{}: top-level statement outside a function: {:?}",
                path.display(),
                i + 1,
                truncate(line, 50)
            ));
        }
        depth += brace_delta(line);
        if depth < 0 {
            depth = 0;
        }
    }
    warnings
}

/// Net change in brace depth on a line, ignoring braces inside comments.
fn brace_delta(line: &str) -> i32 {
    let code = line.split('#').next().unwrap_or("");
    code.chars().fold(0, |acc, c| match c {
        '{' => acc + 1,
        '}' => acc - 1,
        _ => acc,
    })
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n).collect();
        t.push('…');
        t
    }
}

/// List all package names present on disk, sorted.
pub fn list_all() -> Result<Vec<String>> {
    let dir = paths::packages_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names: Vec<String> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    Ok(names)
}
