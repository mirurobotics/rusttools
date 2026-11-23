mod fieldassert;
mod funclen;

// standard crates
use std::path::{Path, PathBuf};

// external crates
use clap::Args;

#[derive(Args, Default)]
pub struct Limits {
    /// Minimum field-asserts on the same receiver to flag (default: 4)
    #[arg(
        long = "assert-threshold",
        value_name = "ASSERT_THRESHOLD",
        default_value_t = 4
    )]
    pub field_assert: usize,

    /// Max non-blank, non-comment body lines per function (0 disables)
    #[arg(
        long = "funclen-threshold",
        value_name = "FUNCLEN_THRESHOLD",
        default_value_t = 50
    )]
    pub funclen: usize,
}

pub const ASSERT_PATH_RULES: &[Rule] = &[fieldassert::check];

pub const PATH_RULES: &[Rule] = &[funclen::check];

pub fn check_files(files: &[PathBuf], rules: &[Rule], limits: &Limits) -> Vec<Diagnostic> {
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

pub fn has_allow_marker(line: &str, rule: &str) -> bool {
    line.contains(&format!("lint:allow({rule})"))
}

pub fn is_code_line(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && !trimmed.starts_with("//")
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

#[cfg(test)]
mod tests {
    use super::*;

    // external crates
    use tempfile::tempdir;

    fn diagnostic_at(input: &Input, line: usize) -> Vec<Diagnostic> {
        vec![Diagnostic {
            file: input.path.to_path_buf(),
            line,
            kind: "test",
            message: String::new(),
        }]
    }

    fn late(input: &Input) -> Vec<Diagnostic> {
        diagnostic_at(input, 9)
    }

    fn early(input: &Input) -> Vec<Diagnostic> {
        diagnostic_at(input, 2)
    }

    #[test]
    fn check_files_sorts_by_file_then_line() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.rs");
        let b = dir.path().join("b.rs");
        std::fs::write(&a, "").unwrap();
        std::fs::write(&b, "").unwrap();
        let limits = Limits::default();

        let diagnostics = check_files(&[b.clone(), a.clone()], &[late, early], &limits);

        let order: Vec<_> = diagnostics.into_iter().map(|d| (d.file, d.line)).collect();
        assert_eq!(order, vec![(a.clone(), 2), (a, 9), (b.clone(), 2), (b, 9)]);
    }
}
