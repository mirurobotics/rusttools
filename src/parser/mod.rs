// internal crates
use crate::rules::ALLOW_MARKER;

/// A single `use` statement (possibly spanning multiple lines).
#[derive(Debug, Clone)]
pub struct UseStatement {
    /// Full text including the trailing semicolon and newline(s)
    pub text: String,
    /// 1-based line number where this statement starts
    pub line: usize,
    /// First path segment after `use` (e.g. "std", "crate", "tokio")
    pub root_crate: String,
    /// Full path used for alphabetical sorting (e.g. "crate::app::state::AppState")
    pub sort_key: String,
    /// Any attribute lines preceding this use (e.g. `#[allow(unused_imports)]`)
    pub attrs: Vec<String>,
}

/// An item in the import block.
#[derive(Debug, Clone)]
pub enum ImportBlockItem {
    Comment {
        text: String,
        #[allow(dead_code)] // used for diagnostics and tests
        line: usize,
    },
    Use(UseStatement),
    BlankLine {
        #[allow(dead_code)] // used for diagnostics and tests
        line: usize,
    },
}

/// The parsed import block at the top of a file.
#[derive(Debug)]
pub struct ImportBlock {
    pub items: Vec<ImportBlockItem>,
    /// 1-based line number of the first line IN the import block
    pub start_line: usize,
    /// 1-based line number of the first line AFTER the import block
    pub end_line: usize,
}

impl ImportBlock {
    pub fn use_statements(&self) -> Vec<&UseStatement> {
        self.items
            .iter()
            .filter_map(|item| match item {
                ImportBlockItem::Use(u) => Some(u),
                _ => None,
            })
            .collect()
    }
}

/// Parse the import block from the top of a Rust source file.
pub fn parse(content: &str) -> ImportBlock {
    let lines: Vec<&str> = content.lines().collect();
    let mut items = Vec::new();
    let mut i = skip_leading_prelude(&lines);
    let mut pending_attrs: Vec<String> = Vec::new();
    let start_line = i + 1;

    while let Some(next) = parse_block_item(&lines, i, &mut items, &mut pending_attrs) {
        i = next;
    }

    // Flush any trailing pending attrs (shouldn't normally happen)
    flush_attrs_as_comments(&mut items, &mut pending_attrs, i + 1);

    // Trim trailing blank lines from the block
    while matches!(items.last(), Some(ImportBlockItem::BlankLine { .. })) {
        items.pop();
    }

    ImportBlock {
        items,
        start_line,
        end_line: i + 1,
    }
}

fn parse_block_item(
    lines: &[&str],
    i: usize,
    items: &mut Vec<ImportBlockItem>,
    pending_attrs: &mut Vec<String>,
) -> Option<usize> {
    let line = *lines.get(i)?;
    let trimmed = line.trim();

    // Blank line
    if trimmed.is_empty() {
        // Flush any pending attrs as comments (they weren't followed by a use)
        flush_attrs_as_comments(items, pending_attrs, i);
        items.push(ImportBlockItem::BlankLine { line: i + 1 });
        return Some(i + 1);
    }

    // Comment line (// ...)
    // Only accept group header comments as part of the import block.
    // Any other comment (like `// === section ===`) ends the block.
    if trimmed.starts_with("//") {
        if !is_group_header_comment(trimmed) {
            return None;
        }
        push_header_comment(items, pending_attrs, line, i);
        return Some(i + 1);
    }

    // Attribute line (#[...]) — only if followed by a use statement
    // (skipping further attrs in the lookahead)
    if trimmed.starts_with("#[") {
        if !attrs_lead_to_use(lines, i) {
            return None;
        }
        pending_attrs.push(line.to_string());
        return Some(i + 1);
    }

    // use or pub use statement
    if trimmed.starts_with("use ") || trimmed.starts_with("pub use ") {
        let (stmt, next) = parse_use_statement(lines, i, pending_attrs);
        items.push(ImportBlockItem::Use(stmt));
        return Some(next);
    }

    // Anything else ends the import block
    None
}

fn push_header_comment(
    items: &mut Vec<ImportBlockItem>,
    pending_attrs: &mut Vec<String>,
    line: &str,
    i: usize,
) {
    flush_attrs_as_comments(items, pending_attrs, i);
    items.push(ImportBlockItem::Comment {
        text: line.to_string(),
        line: i + 1,
    });
}

fn flush_attrs_as_comments(
    items: &mut Vec<ImportBlockItem>,
    pending_attrs: &mut Vec<String>,
    line: usize,
) {
    for attr in pending_attrs.drain(..) {
        items.push(ImportBlockItem::Comment { text: attr, line });
    }
}

/// Consume a (possibly multi-line) use statement starting at `start`.
/// Returns the parsed statement and the index of the first line after it.
fn parse_use_statement(
    lines: &[&str],
    start: usize,
    pending_attrs: &mut Vec<String>,
) -> (UseStatement, usize) {
    let first_line = lines[start];
    let mut text = String::new();
    let mut brace_depth: i32 = 0;
    let mut i = start;

    loop {
        let l = lines[i];
        text.push_str(l);
        text.push('\n');

        brace_depth += brace_delta(l);
        i += 1;

        // Statement ends when we hit a semicolon and brace depth is 0
        if l.trim().ends_with(';') && brace_depth <= 0 {
            break;
        }
        if i >= lines.len() {
            break;
        }
    }

    let (root_crate, sort_key) = extract_path_info(first_line);
    let stmt = UseStatement {
        text,
        line: start + 1,
        root_crate,
        sort_key,
        attrs: std::mem::take(pending_attrs),
    };
    (stmt, i)
}

fn brace_delta(line: &str) -> i32 {
    line.chars()
        .map(|ch| match ch {
            '{' => 1,
            '}' => -1,
            _ => 0,
        })
        .sum()
}

fn skip_leading_prelude(lines: &[&str]) -> usize {
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() || is_module_declaration(trimmed) || is_allow_marker(trimmed) {
            i += 1;
            continue;
        }
        break;
    }
    i
}

fn is_module_declaration(trimmed: &str) -> bool {
    let Some(stmt) = trimmed.strip_suffix(';') else {
        return false;
    };

    let parts: Vec<&str> = stmt.split_whitespace().collect();
    match parts.as_slice() {
        ["mod", _] => true,
        [visibility, "mod", _] => visibility.starts_with("pub"),
        _ => false,
    }
}

fn is_allow_marker(trimmed: &str) -> bool {
    trimmed.starts_with("//") && trimmed.contains(ALLOW_MARKER)
}

/// Check if a comment line looks like a group header.
fn is_group_header_comment(trimmed: &str) -> bool {
    let t = trimmed.to_lowercase();
    t.starts_with("// standard")
        || t.starts_with("// internal")
        || t.starts_with("// external")
        || t.starts_with("// std ")
}

/// Look ahead from position `start` to see if #[attr] lines lead to a use statement.
fn attrs_lead_to_use(lines: &[&str], start: usize) -> bool {
    let mut j = start;
    while j < lines.len() {
        let t = lines[j].trim();
        if t.starts_with("#[") {
            j += 1;
            continue;
        }
        return t.starts_with("use ") || t.starts_with("pub use ");
    }
    false
}

/// Extract root_crate and sort_key from the first line of a use statement.
fn extract_path_info(line: &str) -> (String, String) {
    let trimmed = line.trim();

    // Strip `pub use ` or `use `
    let path_part = if let Some(rest) = trimmed.strip_prefix("pub use ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("use ") {
        rest
    } else {
        trimmed
    };

    // The sort key is the full path (without trailing ; or {... content)
    // For "crate::mqtt::{" → sort_key = "crate::mqtt"
    // For "std::sync::Arc;" → sort_key = "std::sync::Arc"
    let sort_key = path_part
        .trim_end_matches(';')
        .trim_end_matches('{')
        .trim_end_matches("::")
        .trim()
        .to_string();

    // root_crate is the first path segment
    let root_crate = sort_key.split("::").next().unwrap_or("").to_string();

    (root_crate, sort_key)
}

#[cfg(test)]
mod tests;
