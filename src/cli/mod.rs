//! CLI surface and dispatch.
//!
//! Grammar is kubectl-style: `duh VERB RESOURCE [NAME]`. The CRUD verbs
//! (`get`/`create`/`edit`/`delete`/`describe`) take a positional [`Resource`];
//! package lifecycle ops are flat top-level verbs (`enable`, `sync`, …).

mod complete;
mod create;
mod delete;
mod describe;
mod doctor;
mod edit;
mod editor;
mod get;
mod init;
mod man;
mod open;
mod pkgops;
mod resource;
mod ssh;
mod status;
mod uninstall;
mod upgrade;
mod use_pkg;
mod where_cmd;

use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_complete::engine::ArgValueCandidates;
use resource::Resource;

#[derive(Parser)]
#[command(
    name = "duh",
    version,
    about = "Inject your shell config (aliases, exports, functions) everywhere",
    long_about = "duh manages shell aliases, exports, and functions in TOML packages \
                  and injects them into your shell — fast, direnv-style.\n\n\
                  Grammar is kubectl-style: `duh <verb> <resource> [name]`, e.g. \
                  `duh create alias gs 'git status'` or `duh get alias`.\n\n\
                  Quick start: add `eval \"$(duh init)\"` to your shell rc.\n\
                  `init` wires duh into your rc once; `inject` is the script that \
                  wiring runs on each shell start (you rarely call it directly)."
)]
pub struct Cli {
    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,
    /// Plain output: no color and ASCII-only glyphs (max compatibility)
    #[arg(long, global = true)]
    plain: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List resources (bare = everything; `get pkg` lists packages)
    Get {
        /// alias | export | fn | pkg | gitalias (omit to list all)
        resource: Option<Resource>,
        /// Show a single item by name (delegates to `describe`)
        #[arg(add = ArgValueCandidates::new(complete::resource_name))]
        name: Option<String>,
        /// Show only this package
        #[arg(short, long, add = ArgValueCandidates::new(complete::packages))]
        package: Option<String>,
        /// Output machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Create an alias, export, function, package, or git alias
    Create {
        /// alias | export | fn | pkg | gitalias
        resource: Resource,
        /// Name of the new resource
        name: String,
        /// Value (required for alias/export/gitalias; omit for fn/pkg)
        value: Option<String>,
        /// Flag an alias/export as safe to inject over SSH
        #[arg(long)]
        ssh_safe: bool,
        /// Target package (defaults to the default package)
        #[arg(short, long, add = ArgValueCandidates::new(complete::packages))]
        package: Option<String>,
        /// For `create pkg`: clone from this git URL instead of an empty package
        #[arg(long)]
        remote: Option<String>,
    },
    /// Edit a function or a package's db.toml in $EDITOR
    Edit {
        /// fn | pkg
        resource: Resource,
        /// Name (function name; for pkg, the package — defaults to the default package)
        #[arg(add = ArgValueCandidates::new(complete::resource_name))]
        name: Option<String>,
        /// Target package (for `edit fn`)
        #[arg(short, long, add = ArgValueCandidates::new(complete::packages))]
        package: Option<String>,
    },
    /// Delete an alias, export, function, package, or git alias
    Delete {
        /// alias | export | fn | pkg | gitalias
        resource: Resource,
        /// Name to delete
        #[arg(add = ArgValueCandidates::new(complete::resource_name))]
        name: String,
        /// Target package (defaults to the default package)
        #[arg(short, long, add = ArgValueCandidates::new(complete::packages))]
        package: Option<String>,
    },
    /// Show full detail for one item (function doc, package contents, …)
    Describe {
        /// alias | export | fn | pkg | gitalias
        resource: Resource,
        /// Name to describe
        #[arg(add = ArgValueCandidates::new(complete::resource_name))]
        name: String,
        /// Restrict the search to this package
        #[arg(short, long, add = ArgValueCandidates::new(complete::packages))]
        package: Option<String>,
        /// Output machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Show or set the default package that `create`/`delete` write to
    Use {
        /// Package to make default (omit to print the current default)
        #[arg(add = ArgValueCandidates::new(complete::packages))]
        pkg: Option<String>,
    },
    /// Enable a package for injection
    Enable {
        #[arg(add = ArgValueCandidates::new(complete::packages))]
        pkg: String,
    },
    /// Disable a package
    Disable {
        #[arg(add = ArgValueCandidates::new(complete::packages))]
        pkg: String,
    },
    /// Rename a local package
    Rename {
        #[arg(add = ArgValueCandidates::new(complete::packages))]
        old: String,
        new: String,
    },
    /// Pull updates for all enabled remote packages
    Sync,
    /// Commit and push local changes of a package
    Push {
        #[arg(add = ArgValueCandidates::new(complete::packages))]
        pkg: String,
    },
    /// Export a package to a .tar.gz (share without git)
    Export {
        #[arg(add = ArgValueCandidates::new(complete::packages))]
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
        #[arg(add = ArgValueCandidates::new(complete::packages))]
        package: Option<String>,
    },
    /// Diagnose your duh setup (shell wiring, packages, conflicts, git includes)
    Doctor,
    /// Print where duh stores everything (data, config, cache, packages…)
    Where,
    /// Render the man page (roff) to stdout
    Man,
    /// Emit the generated alias/export/function script (run on every shell start
    /// by the rc wiring; you rarely call this directly)
    Inject {
        /// Suppress comments (recommended for rc files)
        #[arg(long)]
        quiet: bool,
    },
    /// direnv-style change check; with --hook, prints a reload command if stale
    Status {
        /// Per-prompt mode: stat-only, prints reload command when stale
        #[arg(long)]
        hook: bool,
        /// Output machine-readable JSON (ignored with --hook)
        #[arg(long)]
        json: bool,
    },
    /// Print the one-time shell rc wiring (run once: add `eval "$(duh init)"`)
    Init {
        /// Target shell (auto-detected from $SHELL if omitted)
        #[arg(long)]
        shell: Option<init::Shell>,
    },
    /// SSH to a host with your config injected
    Ssh {
        /// Target host (e.g. user@host)
        #[arg(add = ArgValueCandidates::new(complete::ssh_hosts))]
        host: String,
        /// Remove the injected snippet from the remote after the session
        #[arg(long)]
        cleanup: bool,
        /// Extra args passed through to ssh (after `--`)
        #[arg(last = true)]
        ssh_args: Vec<String>,
    },
    /// Update duh to the latest release (downloads + verifies + swaps the binary)
    Upgrade {
        /// Only report whether an update is available; don't install
        #[arg(long)]
        check: bool,
    },
    /// Remove duh: deletes the binary, cache, and (with confirmation) packages
    Uninstall {
        /// Skip confirmation prompts (keeps packages unless --purge)
        #[arg(long, short)]
        yes: bool,
        /// Also delete all local packages and config without prompting
        #[arg(long)]
        purge: bool,
    },
}

impl Cli {
    pub fn dispatch(self) -> Result<()> {
        crate::ui::init(self.no_color, self.plain);
        // The per-prompt hook must stay stat-only: never bootstrap there.
        let skip_bootstrap = matches!(
            self.command,
            Command::Status { hook: true, .. }
                | Command::Uninstall { .. }
                | Command::Upgrade { .. }
                | Command::Man
        );
        if !skip_bootstrap {
            config::bootstrap()?;
        }
        match self.command {
            Command::Get {
                resource,
                name,
                package,
                json,
            } => get::run(resource, name, package, json),
            Command::Create {
                resource,
                name,
                value,
                ssh_safe,
                package,
                remote,
            } => create::run(resource, name, value, ssh_safe, package, remote),
            Command::Edit {
                resource,
                name,
                package,
            } => edit::run(resource, name, package),
            Command::Delete {
                resource,
                name,
                package,
            } => delete::run(resource, name, package),
            Command::Describe {
                resource,
                name,
                package,
                json,
            } => describe::run(resource, name, package, json),
            Command::Use { pkg } => use_pkg::run(pkg),
            Command::Enable { pkg } => pkgops::set_enabled(&pkg, true),
            Command::Disable { pkg } => pkgops::set_enabled(&pkg, false),
            Command::Rename { old, new } => pkgops::rename(&old, &new),
            Command::Sync => pkgops::sync(),
            Command::Push { pkg } => pkgops::push(&pkg),
            Command::Export { pkg, out } => pkgops::export(&pkg, out),
            Command::Import { file, name } => pkgops::import(&file, name),
            Command::Open { package } => open::run(package),
            Command::Doctor => doctor::run(),
            Command::Where => where_cmd::run(),
            Command::Man => man::run(),
            Command::Inject { quiet } => status::inject(quiet),
            Command::Status { hook, json } => status::status(hook, json),
            Command::Init { shell } => init::run(shell),
            Command::Ssh {
                host,
                cleanup,
                ssh_args,
            } => ssh::run(&host, cleanup, &ssh_args),
            Command::Upgrade { check } => upgrade::run(check),
            Command::Uninstall { yes, purge } => uninstall::run(yes, purge),
        }
    }
}

use crate::config;
