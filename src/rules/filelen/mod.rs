// standard crates
use std::collections::HashSet;

// internal crates
use crate::rules::{Diagnostic, Input};

// external crates
use proc_macro2::{TokenStream, TokenTree};

const NAME: &str = "filelen";

pub fn check(input: &Input) -> Vec<Diagnostic> {
    let max_lines = input.limits.filelen;
    if max_lines == 0 || is_suppressed(input.source) {
        return Vec::new();
    }
    let count = count_code_lines(input.source);
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
        .is_some_and(|line| line.contains("lint:allow(filelen)"))
}

fn count_code_lines(source: &str) -> usize {
    let Ok(tokens) = source.parse::<TokenStream>() else {
        return 0;
    };
    let mut code_lines = HashSet::new();
    collect_code_lines(tokens, &mut code_lines);
    source
        .lines()
        .enumerate()
        .filter(|(index, line)| code_lines.contains(&(index + 1)) && !line.trim().is_empty())
        .count()
}

fn collect_code_lines(tokens: TokenStream, code_lines: &mut HashSet<usize>) {
    for token in tokens {
        if is_doc_comment(&token) {
            continue;
        }
        match token {
            TokenTree::Group(group) => {
                code_lines.insert(group.span_open().start().line);
                code_lines.insert(group.span_close().start().line);
                collect_code_lines(group.stream(), code_lines);
            }
            other => {
                let span = other.span();
                code_lines.extend(span.start().line..=span.end().line);
            }
        }
    }
}

fn is_doc_comment(token: &TokenTree) -> bool {
    token
        .span()
        .source_text()
        .is_some_and(|text| text.starts_with("//") || text.starts_with("/*"))
}
