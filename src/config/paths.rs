//! Filesystem path resolution (XDG via `directories`, overridable for tests).

use anyhow::{bail, Context, Result};
use directories::{BaseDirs, ProjectDirs};
use std::path::PathBuf;

/// Validate a package name before it is ever joined onto a filesystem path.
///
/// Centralized here so every caller (load, remove, push, enable, ssh package
/// selection, prefs `enabled` entries) is covered — preventing path traversal
/// such as `../../etc` reaching `remove_dir_all` or escaping the packages dir.
/// Allows letters, digits, `_`, `-`, and `.` but rejects empties, `-`-leading,
/// any path separator, and the `.`/`..` traversal names.
pub fn validate_package_name(name: &str) -> Result<()> {
    let bad = name.is_empty()
        || name == "."
        || name == ".."
        || name.starts_with('-')
        || name.contains('/')
        || name.contains('\\')
        || name.contains(std::path::MAIN_SEPARATOR)
        || name.chars().any(|c| c.is_control())
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');
    if bad {
        bail!("invalid package name {name:?}: use letters, digits, '_', '-', '.' only");
    }
    Ok(())
}

/// Resolve the data directory holding packages.
///
/// Honors `DUH_DATA_DIR` when set (used by tests for isolation).
pub fn data_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("DUH_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }
    Ok(project_dirs()?.data_dir().to_path_buf())
}

/// Resolve the config directory holding `prefs.toml` and `ssh.toml`.
///
/// Honors `DUH_CONFIG_DIR` when set.
pub fn config_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("DUH_CONFIG_DIR") {
        return Ok(PathBuf::from(dir));
    }
    Ok(project_dirs()?.config_dir().to_path_buf())
}

/// Resolve the cache directory holding the generated inject script and stamps.
///
/// Honors `DUH_CACHE_DIR` when set.
pub fn cache_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("DUH_CACHE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    Ok(project_dirs()?.cache_dir().to_path_buf())
}

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("net", "fabou", "duh")
        .context("could not determine home directory for duh paths")
}

/// Directory holding all packages.
pub fn packages_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("packages"))
}

/// Directory for a single package. Validates `name` to block path traversal.
pub fn package_dir(name: &str) -> Result<PathBuf> {
    validate_package_name(name)?;
    Ok(packages_dir()?.join(name))
}

/// `db.toml` path for a package.
pub fn package_db(name: &str) -> Result<PathBuf> {
    Ok(package_dir(name)?.join("db.toml"))
}

/// `functions/` directory for a package.
pub fn package_functions_dir(name: &str) -> Result<PathBuf> {
    Ok(package_dir(name)?.join("functions"))
}

/// Per-package git config file (included into `~/.gitconfig` at inject time).
pub fn package_gitconfig(name: &str) -> Result<PathBuf> {
    Ok(package_dir(name)?.join("gitconfig"))
}

/// The user's `~/.gitconfig`. Honors `DUH_GITCONFIG` (test isolation).
pub fn git_config_path() -> Result<PathBuf> {
    if let Some(p) = std::env::var_os("DUH_GITCONFIG") {
        return Ok(PathBuf::from(p));
    }
    let base = BaseDirs::new().context("could not determine home directory")?;
    Ok(base.home_dir().join(".gitconfig"))
}

/// User preferences file.
pub fn prefs_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("prefs.toml"))
}

/// SSH host config file.
pub fn ssh_config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("ssh.toml"))
}

/// Generated shell script consumed by `eval`.
pub fn inject_script_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("inject.sh"))
}

/// Aggregate change stamp (max mtime of all source files, in nanos).
pub fn inject_stamp_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("inject.stamp"))
}

/// Flat newline-separated list of source files the hook must stat.
pub fn inject_files_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("inject.files"))
}

pub const DEFAULT_PACKAGE: &str = "default";
