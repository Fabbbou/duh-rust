//! `duh use [<pkg>]` — show or set the default (write-target) package.

use crate::config::paths;
use crate::config::prefs::Prefs;
use crate::ui;
use anyhow::{bail, Result};

pub fn run(pkg: Option<String>) -> Result<()> {
    let mut prefs = Prefs::load()?;
    match pkg {
        // No arg: print the current default (scriptable, unstyled).
        None => {
            println!("{}", prefs.packages.default);
        }
        Some(name) => {
            if !paths::package_dir(&name)?.exists() {
                bail!("no package {name:?} (see `duh pkg ls`)");
            }
            prefs.packages.default = name.clone();
            prefs.save()?;
            println!(
                "{}",
                ui::ok(&format!("default package is now {}", ui::header(&name)))
            );
            if !prefs.packages.enabled.iter().any(|p| p == &name) {
                eprintln!(
                    "{}",
                    ui::warn(&format!(
                        "package {name} is not enabled — its entries won't inject until `duh pkg enable {name}`"
                    ))
                );
            }
        }
    }
    Ok(())
}
