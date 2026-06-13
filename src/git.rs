//! Git operations for remote packages (clone/pull/push) via `git2`.

use anyhow::{bail, Context, Result};
use std::path::Path;

/// Reject anything that isn't an explicit remote URL. Blocks `file://` and
/// bare local paths that could exfiltrate or import from unexpected locations.
pub fn validate_url(url: &str) -> Result<()> {
    // Defense-in-depth: no control chars, no option-injection via leading '-'.
    if url.chars().any(|c| c.is_control()) || url.starts_with('-') {
        bail!("refusing url {url:?}: control characters or leading '-' not allowed");
    }
    let ok = url.starts_with("https://")
        || url.starts_with("ssh://")
        || url.starts_with("git://")
        || is_scp_like(url);
    if !ok {
        bail!("refusing url {url:?}: only https://, ssh://, git://, or git@host:path are allowed");
    }
    Ok(())
}

/// Detect `git@github.com:user/repo.git` scp-like syntax.
fn is_scp_like(url: &str) -> bool {
    if url.contains("://") {
        return false;
    }
    match url.split_once(':') {
        Some((host, path)) => host.contains('@') && !path.is_empty() && !path.starts_with('/'),
        None => false,
    }
}

/// Clone `url` into `dest`.
pub fn clone(url: &str, dest: &Path) -> Result<()> {
    validate_url(url)?;
    git2::Repository::clone(url, dest)
        .with_context(|| format!("cloning {url} into {}", dest.display()))?;
    Ok(())
}

/// Fetch origin and fast-forward the current branch.
pub fn pull(repo_path: &Path) -> Result<()> {
    let repo = git2::Repository::open(repo_path)
        .with_context(|| format!("opening repo {}", repo_path.display()))?;
    let mut remote = repo.find_remote("origin").context("no 'origin' remote")?;
    remote
        .fetch(&[] as &[&str], None, None)
        .context("fetching origin")?;

    let fetch_head = repo.find_reference("FETCH_HEAD").context("no FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.is_up_to_date() {
        return Ok(());
    }
    if !analysis.is_fast_forward() {
        bail!("local changes diverge from origin; resolve manually");
    }

    let head = repo.head()?;
    let name = head.name().context("HEAD has no name")?.to_string();
    let mut reference = repo.find_reference(&name)?;
    reference.set_target(fetch_commit.id(), "duh: fast-forward")?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    Ok(())
}

/// Stage all changes, commit, and push to origin.
pub fn commit_and_push(repo_path: &Path, message: &str) -> Result<()> {
    let repo = git2::Repository::open(repo_path)
        .with_context(|| format!("opening repo {}", repo_path.display()))?;

    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let sig = repo
        .signature()
        .or_else(|_| git2::Signature::now("duh", "duh@localhost"))?;

    let parent = repo.head().ok().and_then(|h| h.target());
    let parents: Vec<git2::Commit> = match parent {
        Some(oid) => vec![repo.find_commit(oid)?],
        None => vec![],
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)?;

    let mut remote = repo.find_remote("origin").context("no 'origin' remote")?;
    let head = repo.head()?;
    let head_name = head.name().context("HEAD has no name")?;
    let refspec = format!("{head_name}:{head_name}");
    remote
        .push(&[refspec.as_str()], None)
        .context("pushing to origin")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_remote_urls() {
        for u in [
            "https://github.com/u/r.git",
            "ssh://git@h/r.git",
            "git://h/r",
            "git@github.com:u/r.git",
        ] {
            assert!(validate_url(u).is_ok(), "{u}");
        }
    }

    #[test]
    fn rejects_local_and_file() {
        for u in ["file:///etc/passwd", "/tmp/repo", "./repo", "../x", "repo"] {
            assert!(validate_url(u).is_err(), "{u}");
        }
    }
}
