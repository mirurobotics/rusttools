pub mod fieldassert;
pub mod funclen;

// standard crates
use std::path::{Path, PathBuf};

pub struct Limits {
    pub funclen: usize,
    pub field_assert: usize,
}

pub const ASSERT_PATH_RULES: &[Rule] = &[fieldassert::check];

pub const PATH_RULES: &[Rule] = &[funclen::check];

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
