// standard crates
use std::path::{Path, PathBuf};

// internal crates
use crate::rules::{funcorder::*, Limits};

#[test]
fn callee_above_caller_reports_violation() {
    let source = "\
fn helper() {}

fn run() {
    helper();
}
";
    let violations: Vec<_> = check(source)
        .into_iter()
        .map(|d| (d.file, d.line, d.kind, d.message))
        .collect();
    let expected = (
        PathBuf::from("src/lib.rs"),
        1,
        "funcorder",
        String::from("function `helper` should be below its last caller `run`"),
    );
    assert_eq!(violations, vec![expected]);
}

#[test]
fn names_last_caller_once() {
    let source = "\
fn helper() {
    external();
}

fn first() {
    helper();
}

fn second() {
    helper();
}
";
    assert_eq!(
        messages(source),
        vec!["function `helper` should be below its last caller `second`"]
    );
}

#[test]
fn callee_below_all_callers_allowed() {
    let source = "\
fn run() {
    first();
    second();
}

fn first() {
    second();
}

fn second() {}
";
    assert!(lines(source).is_empty());
}

#[test]
fn function_reference_counts_as_call() {
    let source = "\
fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn run(text: &str) -> bool {
    text.chars().all(is_digit)
}
";
    assert_eq!(lines(source), vec![1]);
}

#[test]
fn calls_in_nested_macros_counted() {
    let source = "\
fn helper() -> u8 {
    1
}

fn nested() -> u8 {
    2
}

fn run() {
    assert_eq!(helper(), 1);
    assert!(matches!(nested(), 2));
}
";
    assert_eq!(lines(source), vec![1, 5]);
}

#[test]
fn locals_paths_and_unparsed_macros_not_counted() {
    let source = "\
fn value() -> u8 {
    1
}

fn helper() {}

fn run(value: u8) -> u8 {
    super::helper();
    skip!(helper =>);
    let total = value;
    total
}
";
    assert!(lines(source).is_empty());
}

#[test]
fn cycle_needs_only_outside_caller_above() {
    let source = "\
fn a() {
    b();
}

fn b() {
    a();
    b();
}

fn entry() {
    a();
}
";
    assert_eq!(lines(source), vec![1]);
}

#[test]
fn method_above_own_type_caller_reported() {
    let source = "\
impl Parser {
    const LIMIT: u8 = 1;

    fn helper(&self) {}

    fn limit() -> u8 {
        1
    }

    fn run(&self) -> u8 {
        self.helper();
        Self::limit()
    }
}
";
    assert_eq!(
        messages(source),
        vec![
            "function `Parser::helper` should be below its last caller `Parser::run`",
            "function `Parser::limit` should be below its last caller `Parser::run`",
        ]
    );
}

#[test]
fn trait_impl_calling_inherent_method_comes_first() {
    let source = "\
impl Parser {
    fn helper(&self) {}
}

impl Clone for Parser {
    fn clone(&self) -> Self {
        self.helper();
        Parser
    }
}
";
    assert_eq!(lines(source), vec![2]);
}

#[test]
fn free_function_called_from_method_reported() {
    let source = "\
fn helper() {}

impl Parser {
    fn run(&self) {
        helper();
    }
}
";
    assert_eq!(lines(source), vec![1]);
}

#[test]
fn methods_resolve_through_own_receiver_only() {
    let source = "\
fn helper() {}

impl Lexer {
    fn helper(&self) {}

    fn other(&self, parser: &Parser) {
        parser.helper();
        Parser::helper(parser);
    }
}

impl Parser {
    fn run(&self) {
        self.helper();
    }
}
";
    assert!(lines(source).is_empty());
}

#[test]
fn impl_for_non_path_type_skipped() {
    let source = "\
fn helper() {}

impl Trait for &Parser {
    fn run(&self) {
        helper();
    }
}
";
    assert!(lines(source).is_empty());
}

#[test]
fn inline_module_checked_as_own_scope() {
    let source = "\
mod other;

fn helper() {}

mod tests {
    fn run() {
        helper();
    }

    fn check() {}

    fn case() {
        check();
    }
}
";
    assert_eq!(lines(source), vec![10]);
}

#[test]
fn const_initializer_is_not_a_caller() {
    let source = "\
const LIMIT: u8 = limit();

const fn limit() -> u8 {
    1
}
";
    assert!(lines(source).is_empty());
}

#[test]
fn allow_marker_above_callee_suppresses() {
    let source = "\
// lint:allow(funcorder)
fn helper() {
    let _ = 1;
}

fn run() {
    helper();
}
";
    assert!(lines(source).is_empty());
}

#[test]
fn allow_marker_on_callee_fn_line_suppresses() {
    let source = "\
fn helper() { // lint:allow(funcorder)
    let _ = 1;
}

fn run() {
    helper();
}
";
    assert!(lines(source).is_empty());
}

#[test]
fn allow_marker_on_caller_drops_its_calls() {
    let source = "\
fn helper() {}

// lint:allow(funcorder)
fn run() {
    helper();
}
";
    assert!(lines(source).is_empty());
}

#[test]
fn allow_marker_inside_body_does_not_suppress() {
    let source = "\
fn helper() {
    // lint:allow(funcorder)
}

fn run() {
    helper();
}
";
    assert_eq!(lines(source), vec![1]);
}

#[test]
fn distant_or_other_allow_marker_does_not_suppress() {
    let source = "\
// lint:allow(funcorder)
#[inline]
fn first() {}

// lint:allow(funclen)
fn second() {}

fn run() {
    first();
    second();
}
";
    assert_eq!(lines(source), vec![3, 6]);
}

fn lines(source: &str) -> Vec<usize> {
    check(source).into_iter().map(|d| d.line).collect()
}

fn messages(source: &str) -> Vec<String> {
    check(source).into_iter().map(|d| d.message).collect()
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
