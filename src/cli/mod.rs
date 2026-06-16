//! CLI surface and dispatch.

mod add;
mod complete;
mod doctor;
mod edit;
mod editor;
mod init;
mod ls;
mod man;
mod open;
mod pkg;
mod rm;
mod ssh;
mod status;
mod uninstall;
mod upgrade;
mod use_pkg;
mod where_cmd;

use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_complete::engine::ArgValueCandidates;

#[derive(Parser)]
#[command(
    name = "duh",
    version,
    about = "Inject your shell config (aliases, exports, functions) everywhere",
    long_about = "duh manages shell aliases, exports, and functions in TOML packages \
                  and injects them into your shell — fast, direnv-style.\n\n\
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
    /// Add an alias, export, or function
    Add {
        #[command(subcommand)]
        what: add::AddCmd,
    },
    /// Remove an alias, export, or function
    Rm {
        #[command(subcommand)]
        what: rm::RmCmd,
    },
    /// List aliases, exports, and functions
    Ls {
        /// Optional filter: alias | export | fn
        kind: Option<ls::Kind>,
        /// Show only this package
        #[arg(short, long, add = ArgValueCandidates::new(complete::packages))]
        package: Option<String>,
        /// Print the full documentation for a single function
        #[arg(short = 'f', long = "fn", add = ArgValueCandidates::new(complete::functions))]
        func: Option<String>,
        /// Output machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Manage packages (remote bundles of config)
    Pkg {
        #[command(subcommand)]
        cmd: pkg::PkgCmd,
    },
    /// Show or set the default package that `add`/`rm` write to
    Use {
        /// Package to make default (omit to print the current default)
        #[arg(add = ArgValueCandidates::new(complete::packages))]
        pkg: Option<String>,
    },
    /// Diagnose your duh setup (shell wiring, packages, conflicts, git includes)
    Doctor,
    /// Print where duh stores everything (data, config, cache, packages…)
    Where,
    /// Open a package folder with your configured tool (vscode, nvim, …)
    Open {
        /// Package to open (defaults to the default package)
        #[arg(add = ArgValueCandidates::new(complete::packages))]
        package: Option<String>,
    },
    /// Edit a package's db.toml in $EDITOR
    Edit {
        /// Package to edit (defaults to the default package)
        #[arg(add = ArgValueCandidates::new(complete::packages))]
        package: Option<String>,
    },
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
            Command::Add { what } => add::run(what),
            Command::Rm { what } => rm::run(what),
            Command::Ls {
                kind,
                package,
                func,
                json,
            } => ls::run(kind, package, func, json),
            Command::Pkg { cmd } => pkg::run(cmd),
            Command::Use { pkg } => use_pkg::run(pkg),
            Command::Doctor => doctor::run(),
            Command::Where => where_cmd::run(),
            Command::Open { package } => open::run(package),
            Command::Edit { package } => edit::run(package),
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
