//! Package model: a named TOML bundle of aliases, exports, and function scripts.

use crate::config::paths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Package {
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

/// Extract the doc comment for a function file: the leading run of `#` comment
/// lines at the top of the file, joined into a one-line summary. Returns the
/// first non-empty comment line (mirrors the old Go duh's doc parsing).
pub fn function_doc(path: &std::path::Path) -> Option<String> {
    let body = fs::read_to_string(path).ok()?;
    for line in body.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(rest) = t.strip_prefix('#') {
            let doc = rest.trim();
            if !doc.is_empty() {
                return Some(doc.to_string());
            }
        } else {
            break; // first non-comment line ends the doc block
        }
    }
    None
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
        if depth == 0 && !line.is_empty() && !line.starts_with('#') && !is_function_header(line) {
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

/// Does this line open a function definition? Matches `name() {`,
/// `name ()`, and `function name`.
fn is_function_header(line: &str) -> bool {
    let l = line.trim();
    if let Some(rest) = l.strip_prefix("function ") {
        return !rest.trim().is_empty();
    }
    // `name()` possibly followed by `{` — find the `(`.
    if let Some(idx) = l.find('(') {
        let name = l[..idx].trim();
        let after = l[idx + 1..].trim_start();
        let valid_name = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if valid_name && after.starts_with(')') {
            return true;
        }
    }
    false
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

/// Ensure the default package and its directory exist.
pub fn ensure_default() -> Result<()> {
    let db = paths::package_db(paths::DEFAULT_PACKAGE)?;
    if !db.exists() {
        let mut pkg = Package::default();
        pkg.metadata.name_origin = paths::DEFAULT_PACKAGE.to_string();
        pkg.save(paths::DEFAULT_PACKAGE)?;
        fs::create_dir_all(paths::package_functions_dir(paths::DEFAULT_PACKAGE)?)?;
    }
    Ok(())
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
