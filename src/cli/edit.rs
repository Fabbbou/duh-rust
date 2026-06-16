//! `duh edit [<pkg>]` — open a package's `db.toml` in `$EDITOR`.

use crate::config::package::Package;
use crate::config::paths;
use crate::config::prefs::Prefs;
use crate::ui;
use anyhow::Result;

pub fn run(package: Option<String>) -> Result<()> {
    let name = match package {
        Some(p) => p,
        None => Prefs::load()?.packages.default,
    };
    // Validates name; create the db.toml if the package doesn't exist yet.
    let db = paths::package_db(&name)?;
    if !db.exists() {
        Package::default().save(&name)?;
    }
    super::editor::open_in_editor(&db)?;
    println!(
        "{}",
        ui::ok(&format!("edited package {}", ui::header(&name)))
    );
    Ok(())
}
