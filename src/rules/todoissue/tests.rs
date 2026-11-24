// standard crates
use std::path::{Path, PathBuf};

// internal crates
use crate::rules::{todoissue::*, Limits};

#[test]
fn todo_without_link_reports_violation() {
    let violations: Vec<_> = check("fn f() {}\n// TODO: handle retries\n")
        .into_iter()
        .map(|d| (d.file, d.line, d.kind, d.message))
        .collect();
    let expected = (
        PathBuf::from("src/lib.rs"),
        2,
        "todoissue",
        String::from("TODO comment must include an issue link (an http or https URL)"),
    );
    assert_eq!(violations, vec![expected]);
}

#[test]
fn fixme_without_link_names_fixme() {
    let messages: Vec<_> = check("// FIXME: flaky\n")
        .into_iter()
        .map(|d| d.message)
        .collect();
    assert_eq!(
        messages,
        vec!["FIXME comment must include an issue link (an http or https URL)"]
    );
}

#[test]
fn todo_with_owner_reported() {
    assert_eq!(lines("// TODO(ben): fix\n"), vec![1]);
}

#[test]
fn link_on_same_line_allowed() {
    let source = "// TODO: fix https://github.com/org/repo/issues/1\n";
    assert!(lines(source).is_empty());
}

#[test]
fn link_on_continuation_line_allowed() {
    let source = "// TODO(ben): clean up\n// See (https://linear.app/org/issue/ENG-1).\n";
    assert!(lines(source).is_empty());
}

#[test]
fn link_scheme_is_case_insensitive() {
    let source = "// TODO:HTTPS://github.com/org/repo/issues/2\n// FIXME: Http://a.io\n";
    assert!(lines(source).is_empty());
}

#[test]
fn link_after_incomplete_scheme_allowed() {
    let source = "// TODO: see http:old|https://example.com/issues/3.\n";
    assert!(lines(source).is_empty());
}

#[test]
fn link_without_dotted_host_not_counted() {
    let source = "// TODO: support https:// URLs\n// and http://localhost too.\n";
    assert_eq!(lines(source), vec![1]);
}

#[test]
fn link_belongs_to_later_todo_only() {
    let source = "// TODO: first\n// TODO: second https://a.io/4\n// TODO: third\n";
    assert_eq!(lines(source), vec![1, 3]);
}

#[test]
fn empty_comment_line_ends_todo() {
    let source = "// TODO: handle retries.\n//\n// See https://a.io/5.\n";
    assert_eq!(lines(source), vec![1]);
}

#[test]
fn code_line_ends_comment_group() {
    let source = "// TODO: fix\nfn f() {}\n// See https://a.io/6.\n";
    assert_eq!(lines(source), vec![1]);
}

#[test]
fn blank_line_ends_comment_group() {
    let source = "// lint:allow(todoissue)\n\n// TODO: fix\n";
    assert_eq!(lines(source), vec![3]);
}

#[test]
fn comment_style_change_ends_comment_group() {
    let source = "// TODO: fix\n/// Wraps https://docs.rs/foo.\nfn f() {}\n";
    assert_eq!(lines(source), vec![1]);
}

#[test]
fn inner_doc_style_change_ends_comment_group() {
    let source = "//! TODO: fix\n// See https://a.io/1\n";
    assert_eq!(lines(source), vec![1]);
}

#[test]
fn inner_doc_continuation_line_allowed() {
    let source = "//! TODO: fix\n//! See https://a.io/1\n";
    assert!(lines(source).is_empty());
}

#[test]
fn extra_slashes_comment_reported() {
    assert_eq!(lines("//// TODO: fix\n"), vec![1]);
}

#[test]
fn bare_extra_slashes_line_ends_todo() {
    let source = "// TODO: a\n////\n// See https://a.io/1\n";
    assert_eq!(lines(source), vec![1]);
}

#[test]
fn extra_slashes_continuation_line_allowed() {
    let source = "// TODO: a\n//// See https://a.io/1\n";
    assert!(lines(source).is_empty());
}

#[test]
fn words_starting_with_marker_not_flagged() {
    let source = "// TODOS are tracked.\n// TODO_LIST is a name.\n// FIXMEs are fine.\n";
    assert!(lines(source).is_empty());
}

#[test]
fn bare_marker_and_space_reported_punctuation_not() {
    let source = "// TODO, it signals intent.\n// TODO. Not one.\n// TODO fix it\n// FIXME\n";
    assert_eq!(lines(source), vec![3, 4]);
}

#[test]
fn doc_list_and_indented_comments_checked() {
    let source = "//! FIXME: a\n\n/// - TODO: b\n/// * TODO: c\nfn f() {\n    // TODO: d\n}\n";
    assert_eq!(lines(source), vec![1, 3, 4, 6]);
}

#[test]
fn allow_marker_suppresses_group() {
    let source = "// TODO: known gap\n// lint:allow(todoissue)\nfn f() {}\n";
    assert!(lines(source).is_empty());
}

#[test]
fn allow_marker_in_other_group_ignored() {
    let source = "// lint:allow(todoissue)\nfn f() {}\n// TODO: fix\n";
    assert_eq!(lines(source), vec![3]);
}

#[test]
fn other_allow_comment_does_not_suppress() {
    let source = "// lint:allow(filelen)\n// TODO: fix\n";
    assert_eq!(lines(source), vec![2]);
}

fn lines(source: &str) -> Vec<usize> {
    check(source).into_iter().map(|d| d.line).collect()
}

fn check(source: &str) -> Vec<Diagnostic> {
    let ast = syn::parse_file(source).unwrap();
    let limits = Limits::default();
    super::check(&Input {
        path: Path::new("src/lib.rs"),
        source,
        ast: &ast,
        limits: &limits,
    })
}
