mod functions;
mod literals;

// internal crates
use crate::rules::{funclen::*, Limits};

fn body(n: usize) -> String {
    "    let x = 1;\n".repeat(n)
}

fn src_fn(n: usize) -> String {
    format!("fn sample() {{\n{}}}\n", body(n))
}

fn check(source: &str, max_lines: usize) -> Vec<Diagnostic> {
    check_file(Path::new("src/lib.rs"), source, max_lines)
}

fn check_file(path: &Path, source: &str, max_lines: usize) -> Vec<Diagnostic> {
    let ast = syn::parse_file(source).unwrap();
    let limits = Limits {
        funclen: max_lines,
        ..Default::default()
    };
    super::check(&Input {
        path,
        source,
        ast: &ast,
        limits: &limits,
    })
}

fn count_lines(source: &str) -> usize {
    let item: syn::ItemFn = syn::parse_str(source).unwrap();
    let lines: Vec<&str> = source.lines().collect();
    count_body_lines(&lines, &item.block)
}
