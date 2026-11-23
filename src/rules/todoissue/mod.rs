// internal crates
use crate::rules::{has_allow_marker, Diagnostic, Input};

const NAME: &str = "todoissue";
const MARKERS: [&str; 2] = ["TODO", "FIXME"];
const MESSAGE: &str = "comment must include an issue link (an http or https URL)";

pub fn check(input: &Input) -> Vec<Diagnostic> {
    let lines: Vec<(usize, &str)> = input.source.lines().enumerate().collect();
    lines
        .split(|(_, line)| comment_style(line).is_none())
        .flat_map(|group| group.chunk_by(is_same_style))
        .filter(|group| !is_suppressed(group))
        .flat_map(unlinked_markers)
        .map(|(index, marker)| Diagnostic {
            file: input.path.to_path_buf(),
            line: index + 1,
            kind: NAME,
            message: format!("{marker} {MESSAGE}"),
        })
        .collect()
}

#[derive(PartialEq)]
enum CommentStyle {
    Line,
    OuterDoc,
    InnerDoc,
}

fn comment_style(line: &str) -> Option<CommentStyle> {
    let body = line.trim_start().strip_prefix("//")?;
    if body.starts_with('!') {
        return Some(CommentStyle::InnerDoc);
    }
    if body.starts_with('/') && !body.starts_with("//") {
        return Some(CommentStyle::OuterDoc);
    }
    Some(CommentStyle::Line)
}

fn is_same_style(a: &(usize, &str), b: &(usize, &str)) -> bool {
    comment_style(a.1) == comment_style(b.1)
}

fn is_suppressed(group: &[(usize, &str)]) -> bool {
    group.iter().any(|(_, line)| has_allow_marker(line, NAME))
}

fn unlinked_markers(group: &[(usize, &str)]) -> Vec<(usize, &'static str)> {
    let mut unlinked = Vec::new();
    let mut open = None;
    for &(index, line) in group {
        let text = line.trim_start_matches(is_comment_prefix);
        let marker = todo_marker(text);
        if text.is_empty() || marker.is_some() {
            unlinked.extend(open.take());
            open = marker.map(|marker| (index, marker));
        }
        if has_link(text) {
            open = None;
        }
    }
    unlinked.extend(open);
    unlinked
}

fn is_comment_prefix(c: char) -> bool {
    matches!(c, '/' | '!' | '*' | '-') || c.is_whitespace()
}

fn todo_marker(text: &str) -> Option<&'static str> {
    MARKERS
        .into_iter()
        .find(|marker| text.strip_prefix(marker).is_some_and(is_marker_end))
}

fn is_marker_end(suffix: &str) -> bool {
    suffix
        .chars()
        .next()
        .is_none_or(|c| matches!(c, ':' | '(') || c.is_whitespace())
}

fn has_link(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower
        .match_indices("http")
        .any(|(start, _)| has_host(&lower[start + 4..]))
}

fn has_host(after_http: &str) -> bool {
    let after_scheme = after_http.strip_prefix('s').unwrap_or(after_http);
    let Some(host) = after_scheme.strip_prefix("://") else {
        return false;
    };
    let end = host.find(|c: char| !is_host_char(c)).unwrap_or(host.len());
    let mut labels = host[..end].split('.').map(str::is_empty);
    labels.next() == Some(false) && labels.next() == Some(false)
}

fn is_host_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')
}

#[cfg(test)]
mod tests;
