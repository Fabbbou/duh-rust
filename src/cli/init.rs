//! `duh init` — print the shell rc snippet that wires duh into the prompt.

use anyhow::Result;
use clap::ValueEnum;

#[derive(Clone, Copy, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
}

pub fn run(shell: Option<Shell>) -> Result<()> {
    let shell = shell.unwrap_or_else(detect);
    let snippet = match shell {
        Shell::Bash => BASH,
        Shell::Zsh => ZSH,
    };
    print!("{snippet}");
    Ok(())
}

fn detect() -> Shell {
    match std::env::var("SHELL") {
        Ok(s) if s.contains("zsh") => Shell::Zsh,
        _ => Shell::Bash,
    }
}

const BASH: &str = r#"# duh shell integration (bash) — add to ~/.bashrc:
#   eval "$(duh init --shell bash)"
eval "$(duh _internal emit --quiet)"
__duh_hook() { eval "$(duh _internal hook)"; }
case "$PROMPT_COMMAND" in
  *__duh_hook*) ;;
  *) PROMPT_COMMAND="__duh_hook${PROMPT_COMMAND:+;$PROMPT_COMMAND}" ;;
esac
# tab completion
source <(COMPLETE=bash duh)
"#;

const ZSH: &str = r#"# duh shell integration (zsh) — add to ~/.zshrc:
#   eval "$(duh init --shell zsh)"
eval "$(duh _internal emit --quiet)"
__duh_hook() { eval "$(duh _internal hook)"; }
autoload -Uz add-zsh-hook
add-zsh-hook precmd __duh_hook
# tab completion
source <(COMPLETE=zsh duh)
"#;
