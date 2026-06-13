//! direnv-style cache: write the generated script plus a stat-only change stamp
//! so the per-prompt hook can detect changes without parsing any TOML.

use crate::config::package::Package;
use crate::config::paths;
use crate::config::prefs::Prefs;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Source files whose mtimes determine staleness: each enabled package's
/// `db.toml` plus every function file.
pub fn source_files() -> Result<Vec<PathBuf>> {
    let prefs = Prefs::load()?;
    let mut files = Vec::new();
    for name in prefs.enabled_existing()? {
        files.push(paths::package_db(&name)?);
        files.extend(Package::function_files(&name)?);
    }
    Ok(files)
}

/// Max mtime across the given files, in nanoseconds since the epoch.
/// Missing files contribute 0. The count of files is folded in so that
/// adding/removing a file (without touching mtimes) still bumps the stamp.
fn stamp_for(files: &[PathBuf]) -> u128 {
    let mut max = 0u128;
    for f in files {
        if let Ok(meta) = fs::metadata(f) {
            if let Ok(modified) = meta.modified() {
                if let Ok(dur) = modified.duration_since(UNIX_EPOCH) {
                    max = max.max(dur.as_nanos());
                }
            }
        }
    }
    // Fold in file count so structural changes register even at equal mtimes.
    max.wrapping_add(files.len() as u128)
}

/// Regenerate cache artifacts: the eval script, the flat file list (for the
/// hook to stat), and the change stamp. Returns the generated script text.
pub fn write(script: &str) -> Result<()> {
    let dir = paths::cache_dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;

    let files = source_files()?;
    let stamp = stamp_for(&files);

    // inject.sh may contain secret exports → keep it owner-only (0600).
    write_private(&paths::inject_script_path()?, script)?;

    let listing = files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n");
    write_private(&paths::inject_files_path()?, &listing)?;
    write_private(&paths::inject_stamp_path()?, &stamp.to_string())?;
    Ok(())
}

/// Write a file with owner-only permissions (0600) on Unix; plain write elsewhere.
fn write_private(path: &std::path::Path, contents: &str) -> Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .with_context(|| format!("writing {}", path.display()))?;
        f.write_all(contents.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

/// Fast path for `status --hook`: stat the cached file list and compare the
/// stamp. No TOML parsing. Returns true when a reload is needed.
pub fn is_stale() -> Result<bool> {
    let stamp_path = paths::inject_stamp_path()?;
    let files_path = paths::inject_files_path()?;
    let (Ok(stored_raw), Ok(listing)) = (
        fs::read_to_string(&stamp_path),
        fs::read_to_string(&files_path),
    ) else {
        return Ok(true); // no cache yet → stale
    };
    let stored: u128 = stored_raw.trim().parse().unwrap_or(0);
    let files: Vec<PathBuf> = listing
        .lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect();
    Ok(stamp_for(&files) != stored)
}

/// Touch helper for tests: returns current time as nanos.
#[allow(dead_code)]
pub fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}
