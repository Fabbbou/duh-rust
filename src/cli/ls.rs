//! `duh ls [alias|export|fn]`

use crate::config::package::Package;
use crate::config::prefs::Prefs;
use anyhow::Result;
use clap::ValueEnum;

#[derive(Clone, Copy, ValueEnum)]
pub enum Kind {
    Alias,
    Export,
    Fn,
}

pub fn run(kind: Option<Kind>) -> Result<()> {
    let prefs = Prefs::load()?;
    let packages = prefs.enabled_existing()?;

    let show_alias = matches!(kind, None | Some(Kind::Alias));
    let show_export = matches!(kind, None | Some(Kind::Export));
    let show_fn = matches!(kind, None | Some(Kind::Fn));

    for name in &packages {
        let pkg = Package::load(name)?;
        let funcs = Package::function_files(name)?;
        if pkg.aliases.is_empty() && pkg.exports.is_empty() && funcs.is_empty() {
            continue;
        }
        println!("[{name}]");
        if show_alias {
            for (k, v) in &pkg.aliases {
                println!("  alias  {k} = {v}");
            }
        }
        if show_export {
            for (k, v) in &pkg.exports {
                println!("  export {k} = {v}");
            }
        }
        if show_fn {
            for f in &funcs {
                if let Some(stem) = f.file_stem().and_then(|s| s.to_str()) {
                    println!("  fn     {stem}");
                }
            }
        }
    }
    Ok(())
}
