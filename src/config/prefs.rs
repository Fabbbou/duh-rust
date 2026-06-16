//! User preferences: which packages are enabled and which is the default.

use crate::config::paths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Prefs {
    #[serde(default = "crate::config::default_schema")]
    pub schema: u32,
    #[serde(default)]
    pub packages: PackagePrefs,
    #[serde(default)]
    pub tools: ToolsPrefs,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            schema: crate::config::SCHEMA_VERSION,
            packages: PackagePrefs::default(),
            tools: ToolsPrefs::default(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ToolsPrefs {
    /// Command used by `duh open` to open a package folder (e.g. "code", "nvim").
    /// Empty → fall back to $EDITOR, then $VISUAL.
    #[serde(default)]
    pub open: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackagePrefs {
    #[serde(default = "default_enabled")]
    pub enabled: Vec<String>,
    #[serde(default = "default_name")]
    pub default: String,
}

fn default_enabled() -> Vec<String> {
    vec![paths::DEFAULT_PACKAGE.to_string()]
}

fn default_name() -> String {
    paths::DEFAULT_PACKAGE.to_string()
}

impl Default for PackagePrefs {
    fn default() -> Self {
        PackagePrefs {
            enabled: default_enabled(),
            default: default_name(),
        }
    }
}

impl Prefs {
    /// Load prefs, returning defaults if the file is absent.
    pub fn load() -> Result<Prefs> {
        let path = paths::prefs_path()?;
        if !path.exists() {
            return Ok(Prefs::default());
        }
        let raw =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let prefs: Prefs =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        crate::config::warn_if_newer(prefs.schema, "prefs.toml");
        Ok(prefs)
    }

    /// Persist prefs, creating the config directory as needed.
    pub fn save(&self) -> Result<()> {
        let dir = paths::config_dir()?;
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = paths::prefs_path()?;
        let raw = toml::to_string_pretty(self).context("serializing prefs")?;
        fs::write(&path, raw).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Enabled packages that also exist on disk, in configured order.
    pub fn enabled_existing(&self) -> Result<Vec<String>> {
        let on_disk = crate::config::package::list_all()?;
        Ok(self
            .packages
            .enabled
            .iter()
            .filter(|p| on_disk.contains(p))
            .cloned()
            .collect())
    }

    pub fn enable(&mut self, name: &str) {
        if !self.packages.enabled.iter().any(|p| p == name) {
            self.packages.enabled.push(name.to_string());
        }
    }

    pub fn disable(&mut self, name: &str) {
        self.packages.enabled.retain(|p| p != name);
    }
}

/// Ensure prefs exist on disk with sane defaults.
pub fn ensure() -> Result<()> {
    let path = paths::prefs_path()?;
    if !path.exists() {
        Prefs::default().save()?;
    }
    Ok(())
}
