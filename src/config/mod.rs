//! Configuration: paths, packages, preferences.

pub mod conflicts;
pub mod funcs;
pub mod gitcfg;
pub mod package;
pub mod paths;
pub mod prefs;

use anyhow::Result;

/// On-disk format version for `prefs.toml` and each package `db.toml`.
/// Files without a `schema` field are treated as v1. 0.9 establishes this
/// versioned format; bump + add migration logic on any breaking change.
pub const SCHEMA_VERSION: u32 = 1;

pub(crate) fn default_schema() -> u32 {
    SCHEMA_VERSION
}

/// Warn (once-ish, to stderr) if a file was written by a newer duh.
pub(crate) fn warn_if_newer(schema: u32, what: &str) {
    if schema > SCHEMA_VERSION {
        eprintln!(
            "{}",
            crate::ui::warn(&format!(
                "{what} uses schema v{schema}, newer than this duh (v{SCHEMA_VERSION}); \
                 update duh (`duh upgrade`) to avoid surprises"
            ))
        );
    }
}

/// First-run bootstrap: ensure prefs exist. The default package is NOT created
/// here — `default` is just a pointer in prefs.toml. Its directory is created
/// lazily the first time something is written to it (see `Package::save`), so
/// duh never resurrects an empty `default/` folder on every command.
pub fn bootstrap() -> Result<()> {
    prefs::ensure()?;
    Ok(())
}
