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
    #[serde(default)]
    pub metadata: Metadata,
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
