//! Shell-safe escaping. SECURITY CRITICAL.
//!
//! Values from config are attacker-influenceable if a user adds a shared
//! package from an untrusted remote. Every value emitted into the generated
//! shell script MUST pass through [`single_quote`], and every name MUST pass
//! [`valid_name`]. Single-quoting in POSIX shells disables ALL expansion
//! (`$`, backticks, `\`, globbing), so the only escape concern is the closing
//! quote itself, handled by the `'\''` idiom.

use anyhow::{bail, Result};

/// Wrap `value` in single quotes, neutralizing every shell metacharacter.
///
/// The single embedded-quote case `'` becomes `'\''`: close the quoted span,
/// emit an escaped literal quote, reopen the span.
pub fn single_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// Validate an alias or export identifier.
///
/// Must be a POSIX-ish name: `^[A-Za-z_][A-Za-z0-9_]*$`. This blocks names
/// carrying shell metacharacters, whitespace, or `=` that could break out of
/// the `alias name=...` / `export NAME=...` construct.
pub fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Validate or error, with a helpful message naming the offender.
pub fn require_valid_name(kind: &str, name: &str) -> Result<()> {
    if !valid_name(name) {
        bail!(
            "invalid {kind} name {name:?}: must match [A-Za-z_][A-Za-z0-9_]* \
             (letters, digits, underscore; not starting with a digit)"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_value_quoted() {
        assert_eq!(single_quote("ls -al"), "'ls -al'");
    }

    #[test]
    fn neutralizes_command_substitution() {
        assert_eq!(single_quote("$(rm -rf /)"), "'$(rm -rf /)'");
        assert_eq!(single_quote("`whoami`"), "'`whoami`'");
        assert_eq!(single_quote("$HOME"), "'$HOME'");
    }

    #[test]
    fn neutralizes_embedded_quote_breakout() {
        // Classic breakout attempt: '; rm -rf / ; '
        let evil = "'; rm -rf / ; '";
        let q = single_quote(evil);
        assert_eq!(q, r#"''\''; rm -rf / ; '\'''"#);
    }

    #[test]
    fn neutralizes_newline() {
        assert_eq!(single_quote("a\nrm -rf /"), "'a\nrm -rf /'");
    }

    #[test]
    fn names_accepted() {
        for n in ["ll", "_x", "FOO_BAR", "a1", "EDITOR"] {
            assert!(valid_name(n), "{n} should be valid");
        }
    }

    #[test]
    fn names_rejected() {
        for n in ["", "1a", "a b", "a=b", "a;b", "a$b", "a-b", "a/b", "$(x)"] {
            assert!(!valid_name(n), "{n} should be invalid");
        }
    }
}
