//! CLI surface and dispatch.

mod add;
mod init;
mod ls;
mod pkg;
mod rm;
mod ssh;
mod status;
mod uninstall;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "duh",
    version,
    about = "Inject your shell config (aliases, exports, functions) everywhere",
    long_about = "duh manages shell aliases, exports, and functions in TOML packages \
                  and injects them into your shell — fast, direnv-style. \
                  Add `eval \"$(duh init)\"` to your shell rc to get started."
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
    },
    /// Manage packages (remote bundles of config)
    Pkg {
        #[command(subcommand)]
        cmd: pkg::PkgCmd,
    },
    /// Emit the shell script to eval (use via `eval "$(duh inject)"`)
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
    /// Force regenerate the cache and emit the inject script
    Reload,
    /// Print the shell rc snippet to wire duh into your prompt
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
            Command::Status { hook: true } | Command::Uninstall { .. }
        );
        if !skip_bootstrap {
            config::bootstrap()?;
        }
        match self.command {
            Command::Add { what } => add::run(what),
            Command::Rm { what } => rm::run(what),
            Command::Ls { kind } => ls::run(kind),
            Command::Pkg { cmd } => pkg::run(cmd),
            Command::Inject { quiet } => status::inject(quiet),
            Command::Status { hook } => status::status(hook),
            Command::Reload => status::reload(),
            Command::Init { shell } => init::run(shell),
            Command::Ssh {
                host,
                cleanup,
                ssh_args,
            } => ssh::run(&host, cleanup, &ssh_args),
            Command::Uninstall { yes, purge } => uninstall::run(yes, purge),
        }
    }
}

use crate::config;
