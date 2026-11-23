mod fieldassert;
mod funclen;

// standard crates
use std::path::{Path, PathBuf};

pub struct Limits {
    pub funclen: usize,
    pub field_assert: usize,
}

pub const ASSERT_PATH_RULES: &[Rule] = &[fieldassert::check];

pub const PATH_RULES: &[Rule] = &[funclen::check];

pub fn check(files: &[PathBuf], rules: &[Rule], limits: &Limits) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for path in files {
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(ast) = syn::parse_file(&source) else {
            continue;
        };
        let input = Input {
            path,
            source: &source,
            ast: &ast,
            limits,
        };
        diagnostics.extend(rules.iter().flat_map(|rule| rule(&input)));
    }
    diagnostics.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    diagnostics
}

pub type Rule = fn(&Input) -> Vec<Diagnostic>;

pub struct Input<'a> {
    pub path: &'a Path,
    pub source: &'a str,
    pub ast: &'a syn::File,
    pub limits: &'a Limits,
}

pub struct Diagnostic {
    pub file: PathBuf,
    pub line: usize,
    pub kind: &'static str,
    pub message: String,
}
