//! Configuration: paths, packages, preferences.

pub mod funcs;
pub mod package;
pub mod paths;
pub mod prefs;

use anyhow::Result;

/// First-run bootstrap: ensure prefs exist. The default package is NOT created
/// here — `default` is just a pointer in prefs.toml. Its directory is created
/// lazily the first time something is written to it (see `Package::save`), so
/// duh never resurrects an empty `default/` folder on every command.
pub fn bootstrap() -> Result<()> {
    prefs::ensure()?;
    Ok(())
}
