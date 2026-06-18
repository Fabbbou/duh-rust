//! The resource word in the kubectl-style `duh VERB RESOURCE [NAME]` grammar.

use clap::ValueEnum;

/// A kind of thing duh manages. Used as the positional `RESOURCE` argument of
/// the CRUD verbs (`get`/`create`/`edit`/`delete`/`describe`).
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Resource {
    /// A shell alias.
    #[value(alias = "aliases", alias = "al")]
    Alias,
    /// An environment export.
    #[value(alias = "exports", alias = "env")]
    Export,
    /// A function script.
    #[value(alias = "function", alias = "functions", alias = "func")]
    Fn,
    /// A package (bundle of config).
    #[value(alias = "package", alias = "packages", alias = "pkgs")]
    Pkg,
    /// A git alias in the package's gitconfig.
    #[value(alias = "git-alias", alias = "git")]
    Gitalias,
}

impl Resource {
    /// Parse a resource word (and its aliases) from a raw CLI token. Used by the
    /// completion engine, which inspects argv rather than parsed clap state.
    pub fn from_token(token: &str) -> Option<Resource> {
        match token {
            "alias" | "aliases" | "al" => Some(Resource::Alias),
            "export" | "exports" | "env" => Some(Resource::Export),
            "fn" | "function" | "functions" | "func" => Some(Resource::Fn),
            "pkg" | "package" | "packages" | "pkgs" => Some(Resource::Pkg),
            "gitalias" | "git-alias" | "git" => Some(Resource::Gitalias),
            _ => None,
        }
    }

    /// Human label for messages.
    pub fn label(self) -> &'static str {
        match self {
            Resource::Alias => "alias",
            Resource::Export => "export",
            Resource::Fn => "function",
            Resource::Pkg => "package",
            Resource::Gitalias => "git alias",
        }
    }
}
