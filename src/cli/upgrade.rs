//! `duh upgrade` — self-update to the latest GitHub release.
//!
//! Uses `curl`/`tar`/`sha256sum` subprocesses (curl is already required to
//! install duh) so it pulls in no extra crates. Downloads the release asset for
//! the current platform, verifies its checksum, and atomically swaps the running
//! binary by renaming a temp file in the same directory.

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

const REPO: &str = "Fabbbou/duh-rust";

pub fn run(check_only: bool) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    let target = asset_target()?;

    let latest_tag = latest_release_tag()?;
    let latest = latest_tag.strip_prefix('v').unwrap_or(&latest_tag);

    // Only upgrade when the published release is strictly newer; never downgrade.
    if !is_newer(latest, current) {
        println!("duh is up to date (v{current})");
        return Ok(());
    }

    if check_only {
        println!("update available: v{current} → {latest_tag} (run `duh upgrade`)");
        return Ok(());
    }

    println!("upgrading duh v{current} → {latest_tag} ({target})…");

    let asset = format!("duh-{target}.tar.gz");
    let base = format!("https://github.com/{REPO}/releases/download/{latest_tag}");
    let tmp = tempfile::tempdir().context("creating temp dir")?;
    let tarball = tmp.path().join(&asset);

    curl_download(&format!("{base}/{asset}"), &tarball)
        .with_context(|| format!("downloading {asset}"))?;

    // Verify checksum. The release names it `duh-<target>.sha256` (NOT
    // `<asset>.sha256`). A missing checksum is a hard failure for a self-update —
    // we won't swap an unverified binary into place.
    let sha_name = format!("duh-{target}.sha256");
    let sha_path = tmp.path().join(&sha_name);
    curl_download(&format!("{base}/{sha_name}"), &sha_path)
        .with_context(|| format!("downloading checksum {sha_name}"))?;
    verify_checksum(&tarball, &sha_path)?;

    // Extract.
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(&tarball)
        .arg("-C")
        .arg(tmp.path())
        .status()
        .context("running tar")?;
    if !status.success() {
        bail!("tar extraction failed");
    }
    let new_bin = tmp.path().join("duh");
    if !new_bin.exists() {
        bail!("extracted archive did not contain a `duh` binary");
    }

    swap_binary(&new_bin)?;
    println!("upgraded to {latest_tag}");
    Ok(())
}

/// Compare dotted numeric versions; true if `candidate` > `current`.
/// Non-numeric components compare as 0 (best-effort, no semver crate).
fn is_newer(candidate: &str, current: &str) -> bool {
    parse_version(candidate) > parse_version(current)
}

fn parse_version(v: &str) -> (u32, u32, u32) {
    let mut it = v
        .split(['.', '-', '+'])
        .map(|p| p.parse::<u32>().unwrap_or(0));
    (
        it.next().unwrap_or(0),
        it.next().unwrap_or(0),
        it.next().unwrap_or(0),
    )
}

/// Map the compile-time platform to a release asset target triple.
fn asset_target() -> Result<String> {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => bail!("no prebuilt binary for arch {other}; build from source"),
    };
    let os = match std::env::consts::OS {
        "linux" => "unknown-linux-gnu",
        "macos" => "apple-darwin",
        other => bail!("no prebuilt binary for OS {other}; build from source"),
    };
    Ok(format!("{arch}-{os}"))
}

/// Fetch the latest release tag via the GitHub API.
fn latest_release_tag() -> Result<String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let out = Command::new("curl")
        .args(["-fsSL", "-H", "User-Agent: duh-upgrade", &url])
        .output()
        .context("running curl (is it installed?)")?;
    if !out.status.success() {
        bail!(
            "could not query latest release: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let body = String::from_utf8_lossy(&out.stdout);
    parse_tag(&body).context("could not parse latest release tag from GitHub response")
}

/// Extract the `tag_name` value from a GitHub release JSON body (no JSON dep).
fn parse_tag(body: &str) -> Option<String> {
    let idx = body.find("\"tag_name\"")?;
    let rest = &body[idx + "\"tag_name\"".len()..];
    let colon = rest.find(':')?;
    let after = &rest[colon + 1..];
    let start = after.find('"')? + 1;
    let end = after[start..].find('"')? + start;
    Some(after[start..end].to_string())
}

fn curl_download(url: &str, dest: &Path) -> Result<()> {
    let status = Command::new("curl")
        .args(["-fsSL", "-H", "User-Agent: duh-upgrade", "-o"])
        .arg(dest)
        .arg(url)
        .status()
        .context("running curl")?;
    if !status.success() {
        bail!("download failed: {url}");
    }
    Ok(())
}

fn verify_checksum(tarball: &Path, sha_path: &Path) -> Result<()> {
    let expected = std::fs::read_to_string(sha_path)?
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    // Prefer sha256sum, fall back to `shasum -a 256` (macOS).
    let actual = sha256_of(tarball)?;
    if expected != actual {
        bail!("checksum mismatch (expected {expected}, got {actual})");
    }
    Ok(())
}

fn sha256_of(path: &Path) -> Result<String> {
    let try_cmd = |bin: &str, args: &[&str]| -> Option<String> {
        let out = Command::new(bin).args(args).arg(path).output().ok()?;
        if !out.status.success() {
            return None;
        }
        String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .next()
            .map(|s| s.to_string())
    };
    try_cmd("sha256sum", &[])
        .or_else(|| try_cmd("shasum", &["-a", "256"]))
        .context("no sha256sum/shasum available to verify download")
}

/// Atomically replace the running binary: write the new one into the same
/// directory (so rename is atomic and on the same filesystem), then rename over.
fn swap_binary(new_bin: &Path) -> Result<()> {
    let dest = std::env::current_exe().context("locating current binary")?;
    let dir = dest.parent().context("binary has no parent dir")?;
    let staged = dir.join(".duh.upgrade.tmp");

    std::fs::copy(new_bin, &staged).with_context(|| {
        format!(
            "writing to {} — no permission? try: sudo duh upgrade, or re-run install.sh",
            dir.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&staged, &dest).with_context(|| {
        let _ = std::fs::remove_file(&staged);
        format!("replacing {}", dest.display())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{asset_target, parse_tag};

    #[test]
    fn parses_tag_name() {
        let body = r#"{"url":"x","tag_name": "v0.2.0", "name":"v0.2.0"}"#;
        assert_eq!(parse_tag(body).as_deref(), Some("v0.2.0"));
    }

    #[test]
    fn missing_tag_is_none() {
        assert_eq!(parse_tag("{}"), None);
    }

    #[test]
    fn version_comparison() {
        use super::is_newer;
        assert!(is_newer("0.3.0", "0.2.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.2.1", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.2.0")); // equal
        assert!(!is_newer("0.2.0", "0.3.0")); // older → not newer
    }

    #[test]
    fn target_is_known_or_errors() {
        // On the platforms we build for this resolves; otherwise it errors
        // cleanly (never panics).
        let _ = asset_target();
    }
}
