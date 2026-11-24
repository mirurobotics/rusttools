// internal crates
use crate::classifier::Classifier;
use crate::config::Config;
use crate::fixer::*;
use crate::parser::parse;

#[test]
fn already_correct_unchanged() {
    let content = "\
// standard crates
use std::sync::Arc;

// internal crates
use crate::app::state::AppState;

// external crates
use tokio::sync::broadcast;

fn main() {}
";
    assert_eq!(fix(content), content);
}

#[test]
fn adds_missing_headers() {
    let content = "\
use std::sync::Arc;
use crate::app::state::AppState;
use tokio::sync::broadcast;

fn main() {}
";
    let result = fix(content);
    assert!(result.contains("// standard crates\n"));
    assert!(result.contains("// internal crates\n"));
    assert!(result.contains("// external crates\n"));
}

#[test]
fn regroups_misplaced_imports() {
    let content = "\
// external crates
use std::sync::Arc;
use tokio::sync::broadcast;

fn main() {}
";
    let result = fix(content);
    // std::sync::Arc should now be under standard, not external
    assert!(result.starts_with("// standard crates\nuse std::sync::Arc;\n"));
    assert!(result.contains("// external crates\nuse tokio::sync::broadcast;\n"));
}

#[test]
fn preserves_rest_of_file() {
    let content = "\
use std::sync::Arc;

fn main() {
    let x = 42;
}
";
    let result = fix(content);
    assert!(result.contains("fn main() {\n    let x = 42;\n}"));
}

#[test]
fn preserves_trailing_newline() {
    let content = "use std::sync::Arc;\n\nfn main() {}\n";
    let result = fix(content);
    assert!(result.ends_with('\n'), "should preserve trailing newline");
}

#[test]
fn no_double_blank_line() {
    // Content where rest starts with a blank line
    let content = "\
use std::sync::Arc;


fn main() {}
";
    let result = fix(content);
    // Should not have three consecutive newlines (double blank line)
    assert!(
        !result.contains("\n\n\n"),
        "should not have double blank line, got:\n{result}"
    );
}

#[test]
fn handles_attributes_on_use() {
    let content = "\
// external crates
#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

fn main() {}
";
    let result = fix(content);
    assert!(
        result.contains("#[allow(unused_imports)]"),
        "should preserve attribute"
    );
    assert!(
        result.contains("use tracing::{debug, error, info, warn};"),
        "should preserve use statement"
    );
}

#[test]
fn empty_file_unchanged() {
    let content = "";
    assert_eq!(fix(content), content);
}

#[test]
fn single_group_file() {
    let content = "\
use crate::app::state::AppState;
use crate::http;

fn main() {}
";
    let result = fix(content);
    assert!(result.starts_with("// internal crates\n"));
    // Should not have other group headers
    assert!(!result.contains("// standard crates"));
    assert!(!result.contains("// external crates"));
}

#[test]
fn merges_split_internal_imports() {
    let content = "\
// internal crates
use crate::filesys::dir::Dir;
use crate::filesys::path::PathExt;
use crate::filesys::{Overwrite, WriteOptions};

fn main() {}
";
    let result = fix(content);
    assert!(
        result.contains("use crate::filesys::{Overwrite, WriteOptions, dir::Dir, path::PathExt};"),
        "expected merged filesys import, got:\n{result}"
    );
}

#[test]
fn rewrites_super_imports_to_absolute_crate_paths() {
    let content = "\
// internal crates
use super::errors::HTTPErr;
use super::{request, ClientI};

fn main() {}
";
    let config = default_config();
    let classifier = Classifier::new(&config);
    let block = parse(content);
    let result = fix_file(
        Path::new("daemon/src/http/deployments.rs"),
        content,
        &block,
        &classifier,
        &config,
    );

    assert!(
        result.contains("use crate::http::{ClientI, errors::HTTPErr, request};"),
        "expected absolute http import, got:\n{result}"
    );
}

#[test]
fn preserves_leading_module_declarations() {
    let content = "\
mod context;
mod render;

use tokio::sync::broadcast;
use std::sync::Arc;

fn main() {}
";
    let result = fix_file(
        Path::new("daemon/src/main.rs"),
        content,
        &parse(content),
        &Classifier::new(&default_config()),
        &default_config(),
    );

    assert!(result.starts_with("mod context;\nmod render;\n\n// standard crates\n"));
    assert!(result.contains("// external crates\nuse tokio::sync::broadcast;\n"));
    assert!(result.contains("fn main() {}"));
}

fn fix(content: &str) -> String {
    let config = default_config();
    let classifier = Classifier::new(&config);
    let block = parse(content);
    fix_file(
        Path::new("daemon/src/test.rs"),
        content,
        &block,
        &classifier,
        &config,
    )
}

fn default_config() -> Config {
    Config {
        internal_crates: vec!["backend_api".to_string()],
        ..Default::default()
    }
}
