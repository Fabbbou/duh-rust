//! Terminal styling for human-facing output.
//!
//! Color is enabled only when stdout is a TTY, `NO_COLOR`/`CLICOLOR` allow it,
//! and `--plain`/`--no-color` were not passed (`console::colors_enabled()`
//! handles the env+TTY part; [`init`] forces it off for the flags). `--plain`
//! also switches Unicode glyphs to ASCII for maximum compatibility.
//!
//! Machine output (`inject`, `status --hook`, `init`, and any `--json`) must NOT
//! use this module — it prints raw so the shell can `eval` it unchanged.

use console::style;
use std::sync::atomic::{AtomicBool, Ordering};

static PLAIN: AtomicBool = AtomicBool::new(false);

/// Configure styling once, from the global flags.
/// `no_color` disables ANSI; `plain` disables ANSI *and* uses ASCII glyphs.
pub fn init(no_color: bool, plain: bool) {
    if no_color || plain {
        console::set_colors_enabled(false);
    }
    PLAIN.store(plain, Ordering::Relaxed);
}

fn plain() -> bool {
    PLAIN.load(Ordering::Relaxed)
}

// --- glyphs (Unicode normally, ASCII under --plain) ------------------------

pub fn dot() -> &'static str {
    if plain() {
        "*"
    } else {
        "●"
    }
}

/// Tree branch connector for a non-last child.
pub fn tee() -> &'static str {
    if plain() {
        "-"
    } else {
        "├─"
    }
}

/// Tree branch connector for the last child.
pub fn ell() -> &'static str {
    if plain() {
        "-"
    } else {
        "└─"
    }
}

/// Vertical continuation (indent under a non-last branch).
pub fn pipe() -> &'static str {
    if plain() {
        " "
    } else {
        "│"
    }
}

pub fn arrow() -> String {
    dim(if plain() { "->" } else { "→" })
}

// --- palette (each returns a String, styled only when color is enabled) ----

/// Package header: bold cyan.
pub fn header(s: &str) -> String {
    style(s).cyan().bold().to_string()
}

/// Secondary / structural text: dim.
pub fn dim(s: &str) -> String {
    style(s).dim().to_string()
}

/// Function names: yellow.
pub fn fn_name(s: &str) -> String {
    style(s).yellow().to_string()
}

/// Script (file) names: blue bold — distinct from function names.
pub fn script_name(s: &str) -> String {
    style(s).blue().bold().to_string()
}

// --- row-type keyword labels (each a distinct color) -----------------------

pub fn lbl_alias() -> String {
    style("alias ").green().to_string()
}
pub fn lbl_export() -> String {
    style("export").magenta().to_string()
}
pub fn lbl_git() -> String {
    style("git   ").red().to_string()
}
pub fn lbl_script() -> String {
    style("script").blue().to_string()
}
pub fn lbl_fn() -> String {
    style("fn").yellow().to_string()
}

/// Aligned field label for describe/card views (cyan).
pub fn field(label: &str) -> String {
    style(label).cyan().to_string()
}

/// The `(default)` package marker: green.
pub fn default_badge() -> String {
    style("(default)").green().to_string()
}

/// ssh-safe badge.
pub fn badge_ssh() -> String {
    let label = if plain() {
        "(ssh-safe)".to_string()
    } else {
        "⬡ ssh-safe".to_string()
    };
    style(label).magenta().to_string()
}

pub fn ok(s: &str) -> String {
    let mark = if plain() { "+" } else { "✓" };
    format!("{} {}", style(mark).green().bold(), s)
}

pub fn warn(s: &str) -> String {
    format!("{} {}", style("warning:").yellow().bold(), s)
}

pub fn err(s: &str) -> String {
    format!("{} {}", style("error:").red().bold(), s)
}

/// State word colored by health: `in sync` green, anything else yellow.
pub fn state(s: &str, good: bool) -> String {
    if good {
        style(s).green().to_string()
    } else {
        style(s).yellow().to_string()
    }
}

/// A present/missing mark for `duh where`.
pub fn mark(present: bool) -> String {
    if present {
        let g = if plain() { "+" } else { "✓" };
        style(g).green().to_string()
    } else {
        let x = if plain() { "x" } else { "✗" };
        style(x).red().to_string()
    }
}
