// standard crates
use std::path::Path;

// internal crates
use crate::classifier::{Classifier, ImportGroup};
use crate::config::Config;
use crate::normalizer;
use crate::parser::{ImportBlock, ImportBlockItem};

#[derive(Debug)]
pub struct Diagnostic {
    pub line: usize,
    pub kind: String,
    pub message: String,
}

/// Check an import block for violations. Returns a list of diagnostics.
pub fn check(
    file: &Path,
    block: &ImportBlock,
    classifier: &Classifier,
    config: &Config,
) -> Vec<Diagnostic> {
    let uses = block.use_statements();

    if uses.is_empty() {
        return vec![];
    }

    let mut diagnostics = Vec::new();

    // Classify all use statements
    let classified: Vec<(ImportGroup, &crate::parser::UseStatement)> =
        uses.iter().map(|u| (classifier.classify(u), *u)).collect();

    // Build what the "ideal" block looks like and compare
    // 1. Check group ordering: no group should appear after a later group
    let mut last_group: Option<ImportGroup> = None;
    for (group, stmt) in &classified {
        if let Some(prev) = last_group {
            if (*group as u8) < (prev as u8) {
                diagnostics.push(Diagnostic {
                    line: stmt.line,
                    kind: "wrong-group-order".to_string(),
                    message: format!(
                        "{} import `{}` appears after {} imports",
                        group, stmt.sort_key, prev
                    ),
                });
            }
        }
        last_group = Some(*group);
    }

    // 2. Check comment headers
    // Sorting within groups is left to `cargo fmt` (reorder_imports).
    // Walk through items and verify each group has its header
    check_headers(block, &classified, config, &mut diagnostics);

    diagnostics.extend(
        normalizer::diagnostics(file, &uses)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                line: diagnostic.line,
                kind: diagnostic.kind.to_string(),
                message: diagnostic.message,
            }),
    );

    diagnostics.sort_by_key(|diagnostic| diagnostic.line);
    diagnostics
}

enum HeaderMatch {
    Found,
    Wrong(String),
    Missing,
}

fn check_headers(
    block: &ImportBlock,
    classified: &[(ImportGroup, &crate::parser::UseStatement)],
    config: &Config,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Determine which groups are present
    let mut groups_present = Vec::new();
    let mut seen = [false; 3];
    for (g, _) in classified {
        let idx = *g as usize;
        if !seen[idx] {
            seen[idx] = true;
            groups_present.push(*g);
        }
    }
    groups_present.sort();

    // For each group present, find its first use statement and check that the
    // preceding comment matches the expected label
    for group in &groups_present {
        let expected_label = match group {
            ImportGroup::Standard => &config.labels.standard,
            ImportGroup::Internal => &config.labels.internal,
            ImportGroup::External => &config.labels.external,
        };

        // Find the first use statement in this group
        let first_stmt = classified.iter().find(|(g, _)| g == group).map(|(_, s)| *s);

        let Some(first) = first_stmt else {
            continue;
        };

        match find_header(block, first.line, expected_label) {
            HeaderMatch::Found => {}
            HeaderMatch::Wrong(found) => diagnostics.push(Diagnostic {
                line: first.line,
                kind: "wrong-header".to_string(),
                message: format!("expected header `{expected_label}`, found `{found}`"),
            }),
            HeaderMatch::Missing => diagnostics.push(Diagnostic {
                line: first.line,
                kind: "missing-header".to_string(),
                message: format!("missing `{expected_label}` header before {group} imports"),
            }),
        }
    }
}

/// Locate the header comment for the use statement starting at `first_line`
/// by walking backward from it, skipping blank lines, until the nearest
/// comment or another use statement.
fn find_header(block: &ImportBlock, first_line: usize, expected_label: &str) -> HeaderMatch {
    let idx = block
        .items
        .iter()
        .position(|item| matches!(item, ImportBlockItem::Use(u) if u.line == first_line));
    let Some(idx) = idx else {
        return HeaderMatch::Missing;
    };

    for item in block.items[..idx].iter().rev() {
        match item {
            ImportBlockItem::BlankLine { .. } => continue,
            ImportBlockItem::Comment { text, .. } => {
                let trimmed = text.trim();
                if trimmed == expected_label {
                    return HeaderMatch::Found;
                }
                if is_group_header(trimmed) {
                    return HeaderMatch::Wrong(trimmed.to_string());
                }
                return HeaderMatch::Missing;
            }
            // hit a use statement from another group
            ImportBlockItem::Use(_) => return HeaderMatch::Missing,
        }
    }

    HeaderMatch::Missing
}

/// Check if a comment looks like a group header.
fn is_group_header(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    t.starts_with("// standard")
        || t.starts_with("// internal")
        || t.starts_with("// external")
        || t.starts_with("// std")
}

#[cfg(test)]
mod tests;
