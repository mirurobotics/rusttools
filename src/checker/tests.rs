// internal crates
use crate::checker::*;
use crate::classifier::Classifier;
use crate::config::Config;
use crate::parser::{parse, UseStatement};

#[test]
fn correct_ordering_no_diagnostics() {
    let content = "\
// standard crates
use std::sync::Arc;

// internal crates
use crate::app::state::AppState;

// external crates
use tokio::sync::broadcast;

fn main() {}
";
    let diags = check_content(content);
    assert!(diags.is_empty(), "expected no diagnostics, got: {diags:?}");
}

#[test]
fn wrong_group_order() {
    let content = "\
// external crates
use tokio::sync::broadcast;

// internal crates
use crate::app::state::AppState;

fn main() {}
";
    let diags = check_content(content);
    assert!(
        diags.iter().any(|d| d.kind == "wrong-group-order"),
        "expected wrong-group-order diagnostic, got: {diags:?}"
    );
}

#[test]
fn missing_header() {
    let content = "\
use crate::app::state::AppState;
use tokio::sync::broadcast;

fn main() {}
";
    let diags = check_content(content);
    let missing: Vec<_> = diags
        .iter()
        .filter(|d| d.kind == "missing-header")
        .collect();
    assert!(
        missing.len() >= 2,
        "expected at least 2 missing-header diagnostics (internal + external), got: {missing:?}"
    );
}

#[test]
fn wrong_header_label() {
    let content = "\
// standard crates
use crate::app::state::AppState;

fn main() {}
";
    let diags = check_content(content);
    assert!(
        diags.iter().any(|d| d.kind == "wrong-header"),
        "expected wrong-header diagnostic, got: {diags:?}"
    );
}

#[test]
fn header_with_blank_line_between() {
    // Tests the bug fix: blank line between header and use should still find header
    let content = "\
// internal crates

use crate::app::state::AppState;

fn main() {}
";
    let diags = check_content(content);
    let missing: Vec<_> = diags
        .iter()
        .filter(|d| d.kind == "missing-header")
        .collect();
    assert!(
        missing.is_empty(),
        "header should be found despite blank line, got: {missing:?}"
    );
}

#[test]
fn single_group_needs_header() {
    let content = "\
use crate::app::state::AppState;
use crate::http;

fn main() {}
";
    let diags = check_content(content);
    assert!(
        diags.iter().any(|d| d.kind == "missing-header"),
        "expected missing-header for internal imports, got: {diags:?}"
    );
}

#[test]
fn no_imports_no_diagnostics() {
    let content = "fn main() {}\n";
    let diags = check_content(content);
    assert!(diags.is_empty(), "expected no diagnostics, got: {diags:?}");
}

#[test]
fn relative_super_import_is_reported() {
    let content = "\
// internal crates
use super::errors::HTTPErr;

fn main() {}
";
    let config = default_config();
    let classifier = Classifier::new(&config);
    let block = parse(content);

    let diags = check(
        Path::new("daemon/src/http/deployments.rs"),
        &block,
        &classifier,
        &config,
    );
    assert!(
        diags
            .iter()
            .any(|diagnostic| diagnostic.kind == "relative-super-import"),
        "expected relative-super-import diagnostic, got: {diags:?}"
    );
}

#[test]
fn split_internal_imports_are_reported() {
    let content = "\
// internal crates
use crate::filesys::dir::Dir;
use crate::filesys::path::PathExt;

fn main() {}
";
    let config = default_config();
    let classifier = Classifier::new(&config);
    let block = parse(content);

    let diags = check(
        Path::new("daemon/src/filesys/file.rs"),
        &block,
        &classifier,
        &config,
    );
    assert!(
        diags
            .iter()
            .any(|diagnostic| diagnostic.kind == "split-internal-imports"),
        "expected split-internal-imports diagnostic, got: {diags:?}"
    );
}

#[test]
fn find_header_missing_when_use_line_not_in_block() {
    let block = parse("use crate::app::state::AppState;\n\nfn main() {}\n");
    assert!(matches!(
        find_header(&block, 999, "// internal crates"),
        HeaderMatch::Missing
    ));
}

#[test]
fn find_header_missing_for_non_header_comment() {
    let block = ImportBlock {
        items: vec![
            ImportBlockItem::Comment {
                text: "#[allow(unused_imports)]".to_string(),
                line: 1,
            },
            ImportBlockItem::Use(UseStatement {
                text: "use crate::app::state::AppState;\n".to_string(),
                line: 2,
                root_crate: "crate".to_string(),
                sort_key: "crate::app::state::AppState".to_string(),
                attrs: Vec::new(),
            }),
        ],
        start_line: 1,
        end_line: 3,
    };
    assert!(matches!(
        find_header(&block, 2, "// internal crates"),
        HeaderMatch::Missing
    ));
}

#[test]
fn group_headers_recognized_for_all_labels() {
    assert!(is_group_header("// standard crates"));
    assert!(is_group_header("// internal crates"));
    assert!(is_group_header("// external crates"));
    assert!(is_group_header("// std imports"));
    assert!(!is_group_header("// ======== SECTION ======== //"));
}

fn check_content(content: &str) -> Vec<Diagnostic> {
    let config = default_config();
    let classifier = Classifier::new(&config);
    let block = parse(content);
    check(Path::new("test.rs"), &block, &classifier, &config)
}

fn default_config() -> Config {
    Config {
        internal_crates: vec!["backend_api".to_string()],
        ..Default::default()
    }
}
