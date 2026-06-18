//! `duh where` — print every filesystem location duh uses.

use crate::config::paths;
use crate::ui;
use anyhow::Result;

pub fn run() -> Result<()> {
    let rows = [
        ("data dir", paths::data_dir()?),
        ("packages", paths::packages_dir()?),
        ("config dir", paths::config_dir()?),
        ("prefs", paths::prefs_path()?),
        ("ssh config", paths::ssh_config_path()?),
        ("cache dir", paths::cache_dir()?),
        ("inject script", paths::inject_script_path()?),
    ];
    let width = rows.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    for (label, path) in rows {
        println!(
            "{} {:<width$}  {}",
            ui::mark(path.exists()),
            label,
            ui::dim(&path.display().to_string())
        );
    }
    println!(
        "\n{}",
        ui::dim("shortcut: `duh-cd` → packages (after a shell reload)")
    );
    Ok(())
}
