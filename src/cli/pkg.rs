//! `duh pkg …` — package lifecycle ops (the non-CRUD verbs). CRUD lives on the
//! top-level verbs (`create pkg` / `get pkg` / `delete pkg`); this is the clap
//! layer over the logic in [`super::pkgops`].

use super::pkgops;
use anyhow::Result;
use clap::Subcommand;
use clap_complete::engine::ArgValueCandidates;

#[derive(Subcommand)]
pub enum PkgCmd {
    /// Enable a package for injection
    Enable {
        #[arg(add = ArgValueCandidates::new(super::complete::packages))]
        pkg: String,
    },
    /// Disable a package
    Disable {
        #[arg(add = ArgValueCandidates::new(super::complete::packages))]
        pkg: String,
    },
    /// Rename a local package
    Rename {
        #[arg(add = ArgValueCandidates::new(super::complete::packages))]
        old: String,
        new: String,
    },
    /// Pull updates for all enabled remote packages
    Sync,
    /// Commit and push local changes of a package
    Push {
        #[arg(add = ArgValueCandidates::new(super::complete::packages))]
        pkg: String,
    },
    /// Export a package to a .tar.gz (share without git)
    Export {
        #[arg(add = ArgValueCandidates::new(super::complete::packages))]
        pkg: String,
        /// Output file (default: ./duh-<pkg>.tar.gz)
        #[arg(long)]
        out: Option<String>,
    },
    /// Import a package from a .tar.gz produced by `export`
    Import {
        file: String,
        /// Local name (default: the archived package name)
        name: Option<String>,
    },
    /// Open a package folder with your configured tool (vscode, nvim, …)
    Open {
        /// Package to open (defaults to the default package)
        #[arg(add = ArgValueCandidates::new(super::complete::packages))]
        package: Option<String>,
    },
}

pub fn run(cmd: PkgCmd) -> Result<()> {
    match cmd {
        PkgCmd::Enable { pkg } => pkgops::set_enabled(&pkg, true),
        PkgCmd::Disable { pkg } => pkgops::set_enabled(&pkg, false),
        PkgCmd::Rename { old, new } => pkgops::rename(&old, &new),
        PkgCmd::Sync => pkgops::sync(),
        PkgCmd::Push { pkg } => pkgops::push(&pkg),
        PkgCmd::Export { pkg, out } => pkgops::export(&pkg, out),
        PkgCmd::Import { file, name } => pkgops::import(&file, name),
        PkgCmd::Open { package } => super::open::run(package),
    }
}
