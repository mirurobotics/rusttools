// standard crates
use std::path::Path;

// internal crates
use crate::classifier::{Classifier, ImportGroup};
use crate::config::Config;
use crate::normalizer;
use crate::parser::{ImportBlock, UseStatement};

/// Given file content, its parsed import block, a classifier and config,
/// produce the fixed file content with correctly grouped and labeled imports.
/// Sorting within groups is left to `cargo fmt` (reorder_imports).
pub fn fix_file(
    file: &Path,
    content: &str,
    block: &ImportBlock,
    classifier: &Classifier,
    config: &Config,
) -> String {
    let uses = block.use_statements();
    if uses.is_empty() {
        return content.to_string();
    }

    let import_block = render_import_block(file, &uses, classifier, config);
    splice_import_block(content, block, &import_block)
}

type Buckets<'a> = (
    Vec<&'a UseStatement>,
    Vec<&'a UseStatement>,
    Vec<&'a UseStatement>,
);

/// Build the new import block text. Within-group order is preserved;
/// sorting is left to `cargo fmt` (reorder_imports).
fn render_import_block(
    file: &Path,
    uses: &[&UseStatement],
    classifier: &Classifier,
    config: &Config,
) -> String {
    let (standard, internal, external) = bucket_by_group(uses, classifier);
    let groups = [
        (render_originals(&standard), &config.labels.standard),
        (
            render_internal_group(file, &internal),
            &config.labels.internal,
        ),
        (render_originals(&external), &config.labels.external),
    ];

    let mut import_block = String::new();
    let mut first_group = true;
    for (stmts, label) in groups {
        if stmts.is_empty() {
            continue;
        }

        if !first_group {
            import_block.push('\n');
        }
        first_group = false;

        import_block.push_str(label);
        import_block.push('\n');

        for stmt in stmts {
            import_block.push_str(&stmt);
        }
    }

    import_block
}

/// Bucket use statements into (standard, internal, external) groups,
/// preserving the original relative order within each group.
fn bucket_by_group<'a>(uses: &[&'a UseStatement], classifier: &Classifier) -> Buckets<'a> {
    let mut standard = Vec::new();
    let mut internal = Vec::new();
    let mut external = Vec::new();

    for stmt in uses {
        match classifier.classify(stmt) {
            ImportGroup::Standard => standard.push(*stmt),
            ImportGroup::Internal => internal.push(*stmt),
            ImportGroup::External => external.push(*stmt),
        }
    }

    (standard, internal, external)
}

/// Replace the file's original import block lines with `import_block`,
/// preserving everything before and after it.
fn splice_import_block(content: &str, block: &ImportBlock, import_block: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let block_start_line_idx = block.start_line.saturating_sub(1);
    let block_end_line_idx = block.end_line.saturating_sub(1);
    let mut output = String::new();

    for line in lines.iter().take(block_start_line_idx.min(lines.len())) {
        output.push_str(line);
        output.push('\n');
    }

    output.push_str(import_block);
    let rest_lines = lines_after_block(&lines, block_end_line_idx);

    // Ensure a blank line separates the import block from the rest
    if !rest_lines.is_empty() {
        output.push('\n');
    }

    for (i, line) in rest_lines.iter().enumerate() {
        output.push_str(line);
        if i < rest_lines.len() - 1 {
            output.push('\n');
        }
    }

    // Preserve trailing newline if original had one
    if content.ends_with('\n') && !output.ends_with('\n') {
        output.push('\n');
    }

    output
}

fn lines_after_block<'a>(lines: &'a [&'a str], block_end_line_idx: usize) -> &'a [&'a str] {
    // Collect the rest of the file (everything after the import block)
    let rest_lines = lines.get(block_end_line_idx..).unwrap_or(&[]);

    // Skip leading blank lines from rest to avoid double blank lines
    let rest_skip = rest_lines
        .iter()
        .position(|l| !l.is_empty())
        .unwrap_or(rest_lines.len());
    &rest_lines[rest_skip..]
}

fn render_originals(stmts: &[&UseStatement]) -> Vec<String> {
    stmts
        .iter()
        .map(|stmt| normalizer::render_original(stmt))
        .collect()
}

fn render_internal_group(file: &Path, uses: &[&UseStatement]) -> Vec<String> {
    normalizer::normalize(file, uses)
        .into_iter()
        .map(|stmt| stmt.text)
        .collect()
}

#[cfg(test)]
mod tests;
