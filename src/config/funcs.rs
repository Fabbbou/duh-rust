//! Parse function scripts: list declared functions and their doc comments.
//!
//! Uses the `brush-parser` shell parser to reliably enumerate function
//! definitions (handling `name()` and `function name { … }`, quotes, nested
//! braces, etc.), then associates each with the contiguous `#` comment block
//! directly above its definition — parsers discard comments, so that part is
//! done against the raw source. Falls back to a heuristic header scan if the
//! parser rejects the input (e.g. zsh-only syntax) so listing never fails.

use brush_parser::{ast, Parser, ParserOptions, SourceInfo};
use std::path::Path;

/// A function declared in a script file.
pub struct FuncDef {
    pub name: String,
    /// Doc block (comment lines directly above the def), `#` stripped.
    pub doc: Vec<String>,
    /// 1-based line of the definition (0 if it couldn't be located).
    pub line: usize,
}

impl FuncDef {
    /// First non-empty doc line, for inline summaries.
    pub fn summary(&self) -> Option<&str> {
        self.doc.iter().map(|s| s.as_str()).find(|s| !s.is_empty())
    }
}

/// Parse a function script into its declared functions, in source order.
pub fn parse_functions(path: &Path) -> Vec<FuncDef> {
    let Ok(body) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let lines: Vec<&str> = body.lines().collect();
    let names = parser_names(&body).unwrap_or_else(|| heuristic_names(&lines));

    let mut defs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for name in names {
        if !seen.insert(name.clone()) {
            continue;
        }
        let line = header_line(&name, &lines).unwrap_or(0);
        let doc = if line > 0 {
            doc_above(&lines, line)
        } else {
            Vec::new()
        };
        defs.push(FuncDef { name, doc, line });
    }
    defs.sort_by_key(|d| d.line);
    defs
}

/// Reliable function names via the shell parser; `None` on parse error.
fn parser_names(body: &str) -> Option<Vec<String>> {
    let reader = std::io::Cursor::new(body.as_bytes());
    let mut parser = Parser::new(
        reader,
        &ParserOptions::default(),
        &SourceInfo {
            source: "duh".to_string(),
        },
    );
    let program = parser.parse_program().ok()?;
    let mut names = Vec::new();
    for cc in &program.complete_commands {
        for item in &cc.0 {
            collect_andor(&item.0, &mut names);
        }
    }
    Some(names)
}

fn collect_andor(list: &ast::AndOrList, out: &mut Vec<String>) {
    collect_pipeline(&list.first, out);
    for ao in &list.additional {
        match ao {
            ast::AndOr::And(p) | ast::AndOr::Or(p) => collect_pipeline(p, out),
        }
    }
}

fn collect_pipeline(p: &ast::Pipeline, out: &mut Vec<String>) {
    for cmd in &p.seq {
        if let ast::Command::Function(fd) = cmd {
            out.push(fd.fname.clone());
        }
    }
}

/// Fallback: scan raw lines for function headers.
fn heuristic_names(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|l| name_from_header(l.trim()))
        .collect()
}

/// Extract the function name a line declares, if any. Handles
/// `name()`, `name ()`, and `function name [()] [{]`. Public so the lint in
/// `package.rs` shares one definition of "this line opens a function".
pub fn name_from_header(line: &str) -> Option<String> {
    let l = line.trim();
    if l.starts_with('#') {
        return None;
    }
    if let Some(rest) = l.strip_prefix("function ") {
        let name: String = rest
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if !name.is_empty() {
            return Some(name);
        }
    }
    if let Some(idx) = l.find('(') {
        let name = l[..idx].trim();
        let after = l[idx + 1..].trim_start();
        let ok = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            && after.starts_with(')');
        if ok {
            return Some(name.to_string());
        }
    }
    None
}

/// 1-based line where `name` is declared.
fn header_line(name: &str, lines: &[&str]) -> Option<usize> {
    lines
        .iter()
        .position(|l| name_from_header(l.trim()).as_deref() == Some(name))
        .map(|i| i + 1)
}

/// Collect the contiguous comment block directly above a 1-based def line.
/// Stops at a blank line, code, or a shebang. Returns lines `#`-stripped,
/// in source order.
fn doc_above(lines: &[&str], def_line: usize) -> Vec<String> {
    let mut doc = Vec::new();
    let mut idx = def_line as isize - 2; // line directly above the def (0-based)
    while idx >= 0 {
        let t = lines[idx as usize].trim();
        if t.starts_with("#!") {
            break;
        }
        match t.strip_prefix('#') {
            Some(rest) => {
                doc.push(rest.trim().to_string());
                idx -= 1;
            }
            None => break,
        }
    }
    doc.reverse();
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".sh").tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn parses_both_forms_with_docs_ignoring_shebang() {
        let f = write_tmp(
            "#!/usr/bin/env bash\n\
             # say hello\n\
             # to the world\n\
             greet() {\n  echo hi\n}\n\
             \n\
             # checkout helper\n\
             function gco {\n  git checkout \"$1\"\n}\n",
        );
        let defs = parse_functions(f.path());
        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].name, "greet");
        assert_eq!(defs[0].summary(), Some("say hello"));
        assert_eq!(defs[0].doc, vec!["say hello", "to the world"]);
        assert_eq!(defs[1].name, "gco");
        assert_eq!(defs[1].summary(), Some("checkout helper"));
    }

    #[test]
    fn no_false_positive_from_braces_or_strings() {
        let f = write_tmp(
            "# real one\nfoo() {\n  echo '() not a function'\n  if true; then echo x; fi\n}\n",
        );
        let defs = parse_functions(f.path());
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "foo");
    }

    #[test]
    fn doc_requires_adjacency() {
        // Blank line between comment and def → no doc.
        let f = write_tmp("# orphan comment\n\nbar() { :; }\n");
        let defs = parse_functions(f.path());
        assert_eq!(defs.len(), 1);
        assert!(defs[0].doc.is_empty());
    }
}
