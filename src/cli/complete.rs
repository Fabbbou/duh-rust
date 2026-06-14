//! Dynamic shell-completion candidate providers (clap_complete engine).
//!
//! Each is best-effort: on any error they return an empty list rather than
//! failing completion. Wired onto args via `#[arg(add = ArgValueCandidates::new(..))]`.

use crate::config::funcs;
use crate::config::package::{self, Package};
use crate::config::paths;
use clap_complete::engine::CompletionCandidate;

/// All package names on disk.
pub fn packages() -> Vec<CompletionCandidate> {
    package::list_all()
        .unwrap_or_default()
        .into_iter()
        .map(CompletionCandidate::new)
        .collect()
}

/// All function names declared across packages.
pub fn functions() -> Vec<CompletionCandidate> {
    let mut out = Vec::new();
    for pkg in package::list_all().unwrap_or_default() {
        for file in Package::function_files(&pkg).unwrap_or_default() {
            for d in funcs::parse_functions(&file) {
                out.push(CompletionCandidate::new(d.name));
            }
        }
    }
    out
}

/// Host keys from `ssh.toml` (`[hosts."..."]`).
pub fn ssh_hosts() -> Vec<CompletionCandidate> {
    let Ok(path) = paths::ssh_config_path() else {
        return Vec::new();
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = raw.parse::<toml::Value>() else {
        return Vec::new();
    };
    value
        .get("hosts")
        .and_then(|h| h.as_table())
        .map(|t| t.keys().map(CompletionCandidate::new).collect())
        .unwrap_or_default()
}
