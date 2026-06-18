//! `duh ssh <host>` — open an SSH session with your config injected.
//!
//! Only aliases and exports are shipped by default (portable, POSIX-safe).
//! Function bodies are only sent when the host opts in via `ssh.toml`.

use crate::config::paths;
use crate::inject::generator::{self, GenOptions};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

#[derive(Debug, Default, Deserialize)]
struct SshConfig {
    #[serde(default)]
    hosts: BTreeMap<String, HostConfig>,
}

#[derive(Debug, Deserialize)]
struct HostConfig {
    #[serde(default)]
    packages: Option<Vec<String>>,
    #[serde(default)]
    inject_functions: bool,
}

fn load_host_config(host: &str) -> Result<Option<HostConfig>> {
    let path = paths::ssh_config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)?;
    let cfg: SshConfig =
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    Ok(cfg
        .hosts
        .into_iter()
        .find(|(k, _)| k == host)
        .map(|(_, v)| v))
}

pub fn run(host: &str, cleanup: bool, ssh_args: &[String]) -> Result<()> {
    if host.is_empty() || host.starts_with('-') {
        bail!("invalid host {host:?}");
    }
    let host_cfg = load_host_config(host)?;
    let (only_packages, include_functions) = match &host_cfg {
        Some(c) => (c.packages.clone(), c.inject_functions),
        None => (None, false),
    };
    // Validate any explicitly-selected package names before they hit the FS.
    if let Some(pkgs) = &only_packages {
        for p in pkgs {
            paths::validate_package_name(p)?;
        }
    }

    let snippet = generator::generate(&GenOptions {
        quiet: true,
        include_functions,
        only_packages,
        // Opt-in allowlist: only entries flagged `ssh-safe` are ever shipped.
        ssh_safe_only: true,
    })?;

    if include_functions {
        eprintln!(
            "{}",
            crate::ui::warn(&format!(
                "inject_functions=true for {host} — ALL function bodies in the selected \
                 packages are shipped unfiltered (the ssh-safe allowlist covers \
                 aliases/exports only). Use only with trusted packages. See docs/ssh.md."
            ))
        );
    }
    if snippet.trim().is_empty() {
        eprintln!(
            "{}",
            crate::ui::dim(
                "note: nothing flagged ssh-safe — connecting without injection.\n  \
                 Flag entries with `duh create alias <n> <v> --ssh-safe`. See `duh where` and docs/ssh.md."
            )
        );
    }

    // Write a local temp file (0600, auto-cleaned), scp it over, then source it.
    let remote_path = "~/.duh_session.sh";
    let mut tmp = tempfile::Builder::new()
        .prefix("duh_ssh_")
        .suffix(".sh")
        .tempfile()
        .context("creating temp snippet")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600))?;
    }
    {
        use std::io::Write;
        tmp.write_all(snippet.as_bytes())?;
        tmp.flush()?;
    }

    // scp the snippet. Host is passed as a discrete arg, never via a shell string.
    let scp_target = format!("{host}:.duh_session.sh");
    let scp = Command::new("scp")
        .arg(tmp.path())
        .arg(&scp_target)
        .status()
        .context("running scp (is it installed?)")?;
    if !scp.success() {
        bail!("scp to {host} failed");
    }

    // Build the remote command: source the snippet into an interactive bash.
    let mut remote_cmd = format!("bash --rcfile {remote_path} -i");
    if cleanup {
        remote_cmd = format!("{remote_cmd}; rm -f {remote_path}");
    }

    let mut cmd = Command::new("ssh");
    cmd.arg("-t");
    cmd.args(ssh_args); // pass-through args (already split by clap)
    cmd.arg(host);
    cmd.arg(remote_cmd);

    let status = cmd.status().context("running ssh (is it installed?)")?;
    if !status.success() {
        // Non-zero is normal when the remote shell exits non-zero; surface code.
        if let Some(code) = status.code() {
            std::process::exit(code);
        }
    }
    Ok(())
}
