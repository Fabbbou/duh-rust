//! CLI surface and dispatch.

mod add;
mod init;
mod ls;
mod open;
mod pkg;
mod rm;
mod ssh;
mod status;
mod uninstall;
mod upgrade;
mod where_cmd;

use anyhow::Result;
use clap::{Parser, Subcommand};

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
        #[arg(short, long)]
        package: Option<String>,
    },
    /// Manage packages (remote bundles of config)
    Pkg {
        #[command(subcommand)]
        cmd: pkg::PkgCmd,
    },
    /// Print where duh stores everything (data, config, cache, packages…)
    Where,
    /// Open a package folder with your configured tool (vscode, nvim, …)
    Open {
        /// Package to open (defaults to the default package)
        package: Option<String>,
    },
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
        // The per-prompt hook must stay stat-only: never bootstrap there.
        let skip_bootstrap = matches!(
            self.command,
            Command::Status { hook: true } | Command::Uninstall { .. } | Command::Upgrade { .. }
        );
        if !skip_bootstrap {
            config::bootstrap()?;
        }
        match self.command {
            Command::Add { what } => add::run(what),
            Command::Rm { what } => rm::run(what),
            Command::Ls { kind, package } => ls::run(kind, package),
            Command::Pkg { cmd } => pkg::run(cmd),
            Command::Where => where_cmd::run(),
            Command::Open { package } => open::run(package),
            Command::Inject { quiet } => status::inject(quiet),
            Command::Status { hook } => status::status(hook),
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
