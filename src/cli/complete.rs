//! Dynamic shell-completion candidate providers (clap_complete engine).
//!
//! Each is best-effort: on any error they return an empty list rather than
//! failing completion. Wired onto args via `#[arg(add = ArgValueCandidates::new(..))]`.

use super::resource::Resource;
use crate::config::funcs;
use crate::config::gitcfg;
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

/// Name completion for the CRUD verbs' positional `NAME`. The completion engine
/// gives us no parsed state, so we scan argv for the resource word and return the
/// matching set of names (packages / functions / alias names / …). Best-effort.
pub fn resource_name() -> Vec<CompletionCandidate> {
    let resource = std::env::args().find_map(|a| Resource::from_token(&a));
    match resource {
        Some(Resource::Pkg) => packages(),
        Some(Resource::Fn) => functions(),
        Some(Resource::Alias) => entry_names(EntryKind::Alias),
        Some(Resource::Export) => entry_names(EntryKind::Export),
        Some(Resource::Gitalias) => entry_names(EntryKind::Git),
        None => Vec::new(),
    }
}

enum EntryKind {
    Alias,
    Export,
    Git,
}

/// All alias/export/git-alias names across every package on disk.
fn entry_names(kind: EntryKind) -> Vec<CompletionCandidate> {
    let mut out = Vec::new();
    for pkg in package::list_all().unwrap_or_default() {
        match kind {
            EntryKind::Alias => {
                if let Ok(p) = Package::load(&pkg) {
                    out.extend(p.aliases.keys().cloned().map(CompletionCandidate::new));
                }
            }
            EntryKind::Export => {
                if let Ok(p) = Package::load(&pkg) {
                    out.extend(p.exports.keys().cloned().map(CompletionCandidate::new));
                }
            }
            EntryKind::Git => {
                for (k, _) in gitcfg::aliases(&pkg).unwrap_or_default() {
                    out.push(CompletionCandidate::new(k));
                }
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
