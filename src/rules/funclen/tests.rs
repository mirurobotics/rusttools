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

#[test]
fn under_limit_no_violation() {
    assert!(check(&src_fn(2), 3).is_empty());
}

#[test]
fn exactly_at_limit_no_violation() {
    assert!(check(&src_fn(3), 3).is_empty());
}

#[test]
fn over_limit_reports_violation() {
    let violations = check(&src_fn(4), 3);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 1);
    assert_eq!(
        violations[0].message,
        "function `sample` is 4 lines (limit 3)"
    );
}

#[test]
fn blank_lines_not_counted() {
    let src = format!("fn sample() {{\n{}\n\n\n{}}}\n", body(2), body(1));
    assert!(check(&src, 3).is_empty());
}

#[test]
fn comment_lines_not_counted() {
    let src = format!(
        "fn sample() {{\n    // one\n    // two\n    // three\n{}}}\n",
        body(3)
    );
    assert!(check(&src, 3).is_empty());
}

#[test]
fn closure_over_limit_reported_without_name() {
    let src = format!("fn outer() {{\n    let f = || {{\n{}    }};\n}}\n", body(4));
    let violations = check(&src, 3);
    let closure = violations
        .iter()
        .find(|v| v.message.starts_with("closure"))
        .expect("closure violation");
    assert_eq!(closure.message, "closure is 4 lines (limit 3)");
    assert_eq!(closure.line, 2);
}

#[test]
fn impl_method_over_limit_reported() {
    let src = format!(
        "struct S;\nimpl S {{\n    fn method(&self) {{\n{}    }}\n}}\n",
        body(4)
    );
    let violations = check(&src, 3);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].message.starts_with("function `method`"));
}

#[test]
fn trait_default_method_reported_and_bodyless_ignored() {
    let src = format!(
        "trait T {{\n    fn no_body(&self);\n    fn with_body(&self) {{\n{}    }}\n}}\n",
        body(4)
    );
    let violations = check(&src, 3);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].message.starts_with("function `with_body`"));
}

#[test]
fn test_fn_exempt() {
    let src = format!("#[test]\nfn sample() {{\n{}}}\n", body(10));
    assert!(check(&src, 3).is_empty());
}

#[test]
fn tokio_test_fn_exempt() {
    let src = format!("#[tokio::test]\nasync fn sample() {{\n{}}}\n", body(10));
    assert!(check(&src, 3).is_empty());
}

#[test]
fn cfg_test_mod_exempt() {
    let src = format!(
        "#[cfg(test)]\nmod tests {{\n    fn helper() {{\n{}    }}\n}}\n",
        body(10)
    );
    assert!(check(&src, 3).is_empty());
}

#[test]
fn cfg_test_impl_and_trait_exempt() {
    let src = format!(
        "#[cfg(test)]\nimpl S {{\n    fn helper() {{\n{0}    }}\n}}\n\
         #[cfg(test)]\ntrait T {{\n    fn helper() {{\n{0}    }}\n}}\n",
        body(10)
    );
    assert!(check(&src, 3).is_empty());
}

#[test]
fn doc_commented_fn_over_limit_reported() {
    let src = format!("/// Docs.\nfn sample() {{\n{}}}\n", body(4));
    let violations = check(&src, 3);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 2);
}

#[test]
fn cfg_feature_test_fn_exempt() {
    let src = format!(
        "#[cfg(feature = \"test\")]\nfn sample() {{\n{}}}\n",
        body(10)
    );
    assert!(check(&src, 3).is_empty());
}

#[test]
fn tests_path_exempt() {
    let violations = check_file(Path::new("daemon/tests/foo.rs"), &src_fn(10), 3);
    assert!(violations.is_empty());
}

#[test]
fn tests_rs_file_exempt() {
    let violations = check_file(Path::new("src/checker/tests.rs"), &src_fn(10), 3);
    assert!(violations.is_empty());
}

#[test]
fn suppression_on_line_above_fn() {
    let src = format!("// lint:allow(funclen)\nfn sample() {{\n{}}}\n", body(10));
    assert!(check(&src, 3).is_empty());
}

#[test]
fn suppression_trailing_on_fn_line() {
    let src = format!("fn sample() {{ // lint:allow(funclen)\n{}}}\n", body(10));
    assert!(check(&src, 3).is_empty());
}

#[test]
fn other_allow_comment_does_not_suppress() {
    let src = format!(
        "// lint:allow(field-by-field-assert)\nfn sample() {{\n{}}}\n",
        body(10)
    );
    assert_eq!(check(&src, 3).len(), 1);
}

#[test]
fn threshold_zero_disables() {
    assert!(check(&src_fn(100), 0).is_empty());
}
