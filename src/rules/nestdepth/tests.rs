// standard crates
use std::path::{Path, PathBuf};

// internal crates
use crate::rules::{nestdepth::*, Limits};

const DEEP: &str = "fn f() { loop { loop { loop {} } } }\n";

#[test]
fn at_limit_no_violation() {
    let source = "\
fn f(xs: &[u8]) {
    for x in xs {
        if *x > 0 {
            return;
        }
    }
}
";
    assert!(check(source, 2).is_empty());
}

#[test]
fn over_limit_reports_violation_on_nesting_line() {
    let source = "\
fn f(xss: &[Vec<u8>]) {
    for xs in xss {
        for x in xs {
            if *x > 0 {
                return;
            }
        }
    }
}
";
    let violations: Vec<_> = check(source, 2)
        .into_iter()
        .map(|d| (d.file, d.line, d.kind, d.message))
        .collect();
    let expected = (
        PathBuf::from("src/lib.rs"),
        4,
        "nestdepth",
        String::from(
            "nesting depth is 3 (limit 2) — consider early returns or extracting a helper",
        ),
    );
    assert_eq!(violations, vec![expected]);
}

#[test]
fn deeper_nesting_reported_once() {
    let source = "\
fn f() {
    loop {
        loop {
            loop {
                loop {}
            }
        }
    }
}
";
    assert_eq!(lines(source, 2), vec![4]);
}

#[test]
fn sibling_branches_over_limit_each_reported() {
    let source = "\
fn f(n: u8) {
    loop {
        if n == 0 {
            loop {}
        }
        if n == 1 {
            loop {}
        }
    }
}
";
    assert_eq!(lines(source, 2), vec![4, 7]);
}

#[test]
fn match_while_and_loop_count() {
    let source = "\
fn f(n: u8) {
    while n > 0 {
        loop {
            match n {
                _ => {}
            }
        }
    }
}
";
    assert_eq!(lines(source, 2), vec![4]);
}

#[test]
fn match_arm_adds_no_depth() {
    let source = "\
fn f(n: u8) {
    match n {
        0 => {
            if n > 0 {
                loop {}
            }
        }
        _ => {}
    }
}
";
    assert_eq!(lines(source, 2), vec![5]);
}

#[test]
fn else_if_chain_adds_no_depth() {
    let source = "\
fn f(n: u8) {
    if n == 0 {
    } else if n == 1 {
    } else if n == 2 {
        loop {
            loop {}
        }
    }
}
";
    assert_eq!(lines(source, 2), vec![6]);
}

#[test]
fn else_block_counts_like_then_branch() {
    let source = "\
fn f(n: u8) {
    if n == 0 {
    } else {
        for _ in 0..n {
            loop {}
        }
    }
}
";
    assert_eq!(lines(source, 2), vec![5]);
}

#[test]
fn let_else_adds_no_depth() {
    let source = "\
fn f(x: Option<u8>) {
    let Some(n) = x else {
        loop {
            loop {}
        }
    };
    if n > 0 {}
}
";
    assert_eq!(lines(source, 1), vec![4]);
}

#[test]
fn closure_restarts_depth() {
    let source = "\
fn f() {
    loop {
        loop {
            let g = || {
                loop {
                    loop {
                        loop {}
                    }
                }
            };
            g();
        }
    }
}
";
    assert_eq!(lines(source, 2), vec![7]);
}

#[test]
fn async_block_restarts_depth() {
    let source = "\
fn f() {
    loop {
        loop {
            let task = async {
                loop {
                    loop {
                        loop {}
                    }
                }
            };
        }
    }
}
";
    assert_eq!(lines(source, 2), vec![7]);
}

#[test]
fn nested_fn_restarts_depth() {
    let source = "\
fn f() {
    loop {
        loop {
            fn g() {
                loop {
                    loop {}
                }
            }
        }
    }
}
";
    assert!(check(source, 2).is_empty());
}

#[test]
fn impl_method_checked() {
    let source = format!("struct S;\nimpl S {{\n    {DEEP}}}\n");
    assert_eq!(lines(&source, 2), vec![3]);
}

#[test]
fn test_fn_checked() {
    let source = format!("#[cfg(test)]\nmod tests {{\n    #[test]\n    {DEEP}}}\n");
    assert_eq!(lines(&source, 2), vec![4]);
}

#[test]
fn suppression_on_reported_line() {
    let source = "\
fn f() {
    loop {
        loop {
            loop {} // lint:allow(nestdepth)
        }
    }
}
";
    assert!(check(source, 2).is_empty());
}

#[test]
fn suppression_on_line_above() {
    let source = "\
fn f() {
    loop {
        loop {
            // lint:allow(nestdepth)
            loop {}
        }
    }
}
";
    assert!(check(source, 2).is_empty());
}

#[test]
fn marker_on_fn_line_does_not_suppress() {
    let source = "\
fn f() { // lint:allow(nestdepth)
    loop {
        loop {
            loop {}
        }
    }
}
";
    assert_eq!(lines(source, 2), vec![4]);
}

#[test]
fn other_allow_comment_does_not_suppress() {
    let source = format!("// lint:allow(funclen)\n{DEEP}");
    assert_eq!(lines(&source, 2), vec![2]);
}

#[test]
fn threshold_zero_disables() {
    assert!(check(DEEP, 0).is_empty());
}

fn lines(source: &str, max_depth: usize) -> Vec<usize> {
    check(source, max_depth)
        .into_iter()
        .map(|d| d.line)
        .collect()
}

fn check(source: &str, max_depth: usize) -> Vec<Diagnostic> {
    let ast = syn::parse_file(source).unwrap();
    let limits = Limits {
        nestdepth: max_depth,
        ..Default::default()
    };
    super::check(&Input {
        path: Path::new("src/lib.rs"),
        source,
        ast: &ast,
        limits: &limits,
    })
}
