// internal crates
use crate::rules::{has_allow_marker, is_code_line, Diagnostic, Input};

const NAME: &str = "filelen";

pub fn check(input: &Input) -> Vec<Diagnostic> {
    let max_lines = input.limits.filelen;
    if max_lines == 0 || is_suppressed(input.source) {
        return Vec::new();
    }
    let count = input.source.lines().filter(|line| is_code_line(line)).count();
    if count <= max_lines {
        return Vec::new();
    }
    vec![Diagnostic {
        file: input.path.to_path_buf(),
        line: 1,
        kind: NAME,
        message: format!("file is {count} lines (limit {max_lines})"),
    }]
}

fn is_suppressed(source: &str) -> bool {
    source
        .lines()
        .next()
        .is_some_and(|line| has_allow_marker(line, NAME))
}

#[cfg(test)]
mod tests;
