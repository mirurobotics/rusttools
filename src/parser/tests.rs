// internal crates
use crate::parser::*;

#[test]
fn parse_single_line_use() {
    let content = "use std::sync::Arc;\n\nfn main() {}\n";
    let block = parse(content);
    let uses = block.use_statements();
    assert_eq!(uses.len(), 1);
    assert_eq!(uses[0].root_crate, "std");
    assert_eq!(uses[0].sort_key, "std::sync::Arc");
}

#[test]
fn parse_multiline_use() {
    let content =
        "use crate::mqtt::{\n    client::Client,\n    device::Device,\n};\n\nfn main() {}\n";
    let block = parse(content);
    let uses = block.use_statements();
    assert_eq!(uses.len(), 1);
    assert_eq!(uses[0].root_crate, "crate");
    assert_eq!(uses[0].sort_key, "crate::mqtt");
    assert!(uses[0].text.contains("client::Client"));
}

#[test]
fn parse_with_comments_and_groups() {
    let content = "\
// standard crates
use std::sync::Arc;

// internal crates
use crate::app::state::AppState;

// external crates
use tokio::sync::broadcast;

fn main() {}
";
    let block = parse(content);
    let uses = block.use_statements();
    assert_eq!(uses.len(), 3);
    assert_eq!(uses[0].root_crate, "std");
    assert_eq!(uses[1].root_crate, "crate");
    assert_eq!(uses[2].root_crate, "tokio");
    assert_eq!(block.start_line, 1);
}

#[test]
fn parse_empty_file() {
    let block = parse("");
    assert!(block.items.is_empty());
}

#[test]
fn parse_no_imports() {
    let content = "fn main() {}\n";
    let block = parse(content);
    assert!(block.use_statements().is_empty());
}

#[test]
fn parse_pub_use() {
    let content = "pub use self::errors::SyncErr;\n\nfn main() {}\n";
    let block = parse(content);
    let uses = block.use_statements();
    assert_eq!(uses.len(), 1);
    assert_eq!(uses[0].root_crate, "self");
    assert_eq!(uses[0].sort_key, "self::errors::SyncErr");
}

#[test]
fn parse_with_attribute() {
    let content = "#[allow(unused_imports)]\nuse tracing::{debug, error};\n\nfn main() {}\n";
    let block = parse(content);
    let uses = block.use_statements();
    assert_eq!(uses.len(), 1);
    assert_eq!(uses[0].attrs.len(), 1);
    assert!(uses[0].attrs[0].contains("allow(unused_imports)"));
}

#[test]
fn stops_at_separator_comment() {
    let content = "\
use crate::models;

// ======== SECTION ======== //
#[derive(Debug)]
pub struct Foo;
";
    let block = parse(content);
    let uses = block.use_statements();
    assert_eq!(uses.len(), 1);
    assert_eq!(uses[0].root_crate, "crate");
    // The separator comment and derive should NOT be in the block
    assert_eq!(block.end_line, 3); // block ends before separator
}

#[test]
fn stops_at_derive_without_use() {
    let content = "\
use serde::Serialize;

#[derive(Debug)]
pub enum Foo { A, B }
";
    let block = parse(content);
    let uses = block.use_statements();
    assert_eq!(uses.len(), 1);
}

#[test]
fn parse_skips_leading_module_declarations() {
    let content = "\
mod context;
pub(crate) mod render;

// standard crates
use std::sync::Arc;

fn main() {}
";
    let block = parse(content);
    let uses = block.use_statements();

    assert_eq!(uses.len(), 1);
    assert_eq!(uses[0].root_crate, "std");
    assert_eq!(block.start_line, 4);
}
