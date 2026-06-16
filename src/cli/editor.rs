//! Shared `$EDITOR` launcher.

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

/// Open `path` in `$EDITOR` (then `$VISUAL`, then `vi`).
pub fn open_in_editor(path: &Path) -> Result<()> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(&editor)
        .arg(path)
        .status()
        .with_context(|| format!("launching editor {editor}"))?;
    if !status.success() {
        bail!("editor {editor} exited with failure");
    }
    Ok(())
}
