//! `duh where` — print every filesystem location duh uses.

use crate::config::paths;
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
        let mark = if path.exists() { " " } else { "✗" };
        println!("{mark} {label:<width$}  {}", path.display());
    }
    println!("\n(✗ = not created yet)");
    println!("shortcuts: `duh-cd` → packages, `duh-cd-config` → config (after a shell reload)");
    Ok(())
}
