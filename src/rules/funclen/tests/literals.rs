// internal crates
use crate::rules::funclen::tests::*;

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
fn repeat_literals_collapse_and_other_macros_count_every_line() {
    let src = "\
fn sample() {
    let zs = vec![
        0;
        3
    ];
    let ws = [
        0;
        3
    ];
    let s = format!(
        \"{}\",
        1
    );
}
";
    assert_eq!(count_lines(src), 6);
}

#[test]
fn unparseable_vec_counts_every_line() {
    let src = "fn sample() {\n    let ys = vec![\n        1 2,\n    ];\n}\n";
    assert_eq!(count_lines(src), 3);
}

#[test]
fn match_inside_literal_counted() {
    let src = "\
fn sample() {
    let d = D {
        kind: match k {
            1 => \"a\",
            _ => \"b\",
        },
    };
}
";
    assert_eq!(count_lines(src), 6);
}

#[test]
fn match_inside_macro_inside_literal_counted() {
    let src = "\
fn sample() {
    let s = S {
        msg: format!(\"{}\", match k {
            1 => \"a\",
            _ => \"b\",
        }),
    };
}
";
    assert_eq!(count_lines(src), 6);
}

#[test]
fn closure_inside_literal_counted() {
    let src = "fn sample() {\n    let d = D {\n        f: |x| x\n            + 1,\n    };\n}\n";
    assert_eq!(count_lines(src), 4);
}

#[test]
fn closure_inside_vec_counted() {
    let src = "fn sample() {\n    let fs = vec![|x| x\n        + 1];\n}\n";
    assert_eq!(count_lines(src), 2);
}

#[test]
fn block_inside_nested_vec_counted() {
    let src = "\
fn sample() {
    let d = D {
        f: vec![loop {
            break 1;
        }],
    };
}
";
    assert_eq!(count_lines(src), 5);
}

#[test]
fn literal_inside_non_collapsing_vec_collapses() {
    let src = "fn sample() {\n    let v = vec![|| 1, S {\n        a: 1,\n    }];\n}\n";
    assert_eq!(count_lines(src), 1);
}

#[test]
fn code_inside_literal_on_its_closing_line_not_counted() {
    let src = "fn sample() {\n    let xs = [\n        1,\n        2];\n}\n";
    assert_eq!(count_lines(src), 1);
}

#[test]
fn literal_inside_statement_macro_collapses() {
    let src = "fn sample() {\n    assert_eq!(got, S {\n        a: 1,\n    });\n}\n";
    assert_eq!(count_lines(src), 1);
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
