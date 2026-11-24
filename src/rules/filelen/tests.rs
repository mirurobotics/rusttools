// standard crates
use std::path::{Path, PathBuf};

// internal crates
use crate::rules::{filelen::*, Limits};

#[test]
fn under_limit_no_violation() {
    assert!(check(&code(2), 3).is_empty());
}

#[test]
fn exactly_at_limit_no_violation() {
    assert!(check(&code(3), 3).is_empty());
}

#[test]
fn over_limit_reports_violation_on_line_one() {
    let violations: Vec<_> = check(&code(4), 3)
        .into_iter()
        .map(|d| (d.file, d.line, d.kind, d.message))
        .collect();
    let expected = (
        PathBuf::from("src/lib.rs"),
        1,
        "filelen",
        String::from("file is 4 lines (limit 3)"),
    );
    assert_eq!(violations, vec![expected]);
}

#[test]
fn blank_lines_not_counted() {
    let src = format!("{}\n   \n\n{}", code(2), code(1));
    assert!(check(&src, 3).is_empty());
}

#[test]
fn comment_lines_not_counted() {
    let src = format!("// one\n    // two\n/// three\n{}", code(3));
    assert!(check(&src, 3).is_empty());
}

#[test]
fn cfg_test_module_lines_counted() {
    let src = format!("#[cfg(test)]\nmod tests {{\n{}}}\n", code(2));
    let violations = check(&src, 4);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].message, "file is 5 lines (limit 4)");
}

#[test]
fn tests_path_checked() {
    let violations = check_file(Path::new("daemon/tests/foo.rs"), &code(4), 3);
    assert_eq!(violations.len(), 1);
}

#[test]
fn tests_rs_file_checked() {
    let violations = check_file(Path::new("src/checker/tests.rs"), &code(4), 3);
    assert_eq!(violations.len(), 1);
}

#[test]
fn suppression_on_first_line() {
    let src = format!("// lint:allow(filelen)\n{}", code(10));
    assert!(check(&src, 3).is_empty());
}

#[test]
fn suppression_after_first_line_ignored() {
    let src = format!("{}// lint:allow(filelen)\n{}", code(1), code(10));
    assert_eq!(check(&src, 3).len(), 1);
}

#[test]
fn other_allow_comment_does_not_suppress() {
    let src = format!("// lint:allow(funclen)\n{}", code(10));
    assert_eq!(check(&src, 3).len(), 1);
}

#[test]
fn threshold_zero_disables() {
    assert!(check(&code(100), 0).is_empty());
}

fn code(n: usize) -> String {
    "const _: u8 = 0;\n".repeat(n)
}

fn check(source: &str, max_lines: usize) -> Vec<Diagnostic> {
    check_file(Path::new("src/lib.rs"), source, max_lines)
}

fn check_file(path: &Path, source: &str, max_lines: usize) -> Vec<Diagnostic> {
    let ast = syn::parse_file(source).unwrap();
    let limits = Limits {
        filelen: max_lines,
        ..Default::default()
    };
    super::check(&Input {
        path,
        source,
        ast: &ast,
        limits: &limits,
    })
}
