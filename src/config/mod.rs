//! Configuration: paths, packages, preferences.

pub mod package;
pub mod paths;
pub mod prefs;

use anyhow::Result;

/// First-run bootstrap: ensure default package and prefs exist.
pub fn bootstrap() -> Result<()> {
    package::ensure_default()?;
    prefs::ensure()?;
    Ok(())
}
