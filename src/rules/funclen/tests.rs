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
fn test_fn_counted() {
    let src = format!("#[test]\nfn sample() {{\n{}}}\n", body(4));
    assert_eq!(check(&src, 3).len(), 1);
}

#[test]
fn cfg_test_mod_counted() {
    let src = format!(
        "#[cfg(test)]\nmod tests {{\n    fn helper() {{\n{}    }}\n}}\n",
        body(4)
    );
    assert_eq!(check(&src, 3).len(), 1);
}

#[test]
fn doc_commented_fn_over_limit_reported() {
    let src = format!("/// Docs.\nfn sample() {{\n{}}}\n", body(4));
    let violations = check(&src, 3);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 2);
}

#[test]
fn tests_dir_file_counted() {
    let violations = check_file(Path::new("daemon/tests/foo.rs"), &src_fn(4), 3);
    assert_eq!(violations.len(), 1);
}

#[test]
fn tests_rs_file_counted() {
    let violations = check_file(Path::new("src/checker/tests.rs"), &src_fn(4), 3);
    assert_eq!(violations.len(), 1);
}

#[test]
fn multi_line_struct_literal_counts_as_one_line() {
    let src = "\
fn sample() {
    let s = S {
        a: 1,
        b: 2,
    };
    drop(s);
}
";
    assert_eq!(count_lines(src), 2);
}

#[test]
fn multi_line_array_and_vec_literals_count_as_one_line() {
    let src = "\
fn sample() {
    let xs = [
        1,
        2,
    ];
    let ys = vec![
        3,
        4,
    ];
}
";
    assert_eq!(count_lines(src), 2);
    assert_eq!(
        check(src, 1)[0].message,
        "function `sample` is 2 lines (limit 1)"
    );
}

#[test]
fn vec_repeat_collapses_and_other_macros_count_every_line() {
    let src = "\
fn sample() {
    let zs = vec![
        0;
        3
    ];
    let s = format!(
        \"{}\",
        1
    );
}
";
    assert_eq!(count_lines(src), 5);
}

#[test]
fn unparseable_vec_counts_every_line() {
    let src = "fn sample() {\n    let ys = vec![\n        1 2,\n    ];\n}\n";
    assert_eq!(count_lines(src), 3);
}

#[test]
fn block_expressions_inside_literal_counted() {
    let src = "\
fn sample() {
    let d = D {
        kind: match k {
            1 => \"a\",
            _ => \"b\",
        },
        n: loop {
            break 1;
        },
        f: vec![|| 1],
    };
}
";
    assert_eq!(
        check(src, 9)[0].message,
        "function `sample` is 10 lines (limit 9)"
    );
}

#[test]
fn closure_inside_vec_counted() {
    let src = "\
fn sample() {
    let fs = vec![|| {
        let x = 1;
        let y = 2;
    }];
}
";
    assert_eq!(count_lines(src), 4);
}

#[test]
fn lines_around_literal_still_counted() {
    let src = format!(
        "fn sample() {{\n    let xs = [\n        1,\n        2,\n    ];\n{}}}\n",
        body(3)
    );
    assert_eq!(count_lines(&src), 4);
}

#[test]
fn closure_inside_literal_still_checked() {
    let src = format!(
        "fn outer() {{\n    let fs = [|| {{\n{}    }}];\n}}\n",
        body(4)
    );
    let violations = check(&src, 3);
    assert_eq!(violations.len(), 2);
    assert_eq!(violations[1].message, "closure is 4 lines (limit 3)");
}

#[test]
fn code_after_literal_on_closing_line_counted() {
    let src = "\
fn sample() {
    if is_ok(S {
        a: 1,
    }) && ready {
        return;
    }
}
";
    assert_eq!(count_lines(src), 4);
}

#[test]
fn method_or_let_after_literal_on_closing_line_counted() {
    let method = "fn sample() {\n    let s = S {\n        a: 1,\n    }.build();\n}\n";
    let binding = "fn sample() {\n    let x = S {\n        a: 1,\n    }; let y;\n}\n";
    assert_eq!((count_lines(method), count_lines(binding)), (2, 2));
}

#[test]
fn punctuation_and_comment_after_literal_not_counted() {
    let src = "\
fn sample() {
    g(S {
        a: 1,
    }); // note
    h(&[
        1,
    ]);
}
";
    assert_eq!(count_lines(src), 2);
}

#[test]
fn literal_starting_on_closing_line_counted() {
    let src = "\
fn sample() {
    g(A {
        x: 1,
    }, B {
        y: 2,
    });
}
";
    assert_eq!(count_lines(src), 2);
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
