// standard crates
use std::path::Path;

// internal crates
use crate::rules::{Diagnostic, Input};

// external crates
use syn::visit::Visit;

const NAME: &str = "funclen";

/// Scan a source file for functions and closures whose bodies exceed
/// `limits.funclen` non-blank, non-comment lines. A `limits.funclen` of 0 disables the
/// check. Test code (files under a `tests` directory or named `tests.rs`, `#[cfg(test)]` items,
/// `#[test]` functions, `#[cfg(feature = "test")]` items) is exempt.
pub fn check(input: &Input) -> Vec<Diagnostic> {
    if input.limits.funclen == 0 {
        return Vec::new();
    }
    if is_test_path(input.path) {
        return Vec::new();
    }

    let mut visitor = FnVisitor {
        path: input.path,
        lines: input.source.lines().collect(),
        max_lines: input.limits.funclen,
        diagnostics: Vec::new(),
    };
    visitor.visit_file(input.ast);
    visitor.diagnostics
}

fn is_test_path(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "tests.rs")
        || path.components().any(|c| c.as_os_str() == "tests")
}

struct FnVisitor<'a> {
    path: &'a Path,
    lines: Vec<&'a str>,
    max_lines: usize,
    diagnostics: Vec<Diagnostic>,
}

impl FnVisitor<'_> {
    fn check_block(&mut self, block: &syn::Block, anchor: usize, name: Option<String>) {
        if self.is_suppressed(anchor) {
            return;
        }
        let count = count_body_lines(&self.lines, block);
        if count > self.max_lines {
            let target = match name {
                Some(name) => format!("function `{name}`"),
                None => "closure".to_string(),
            };
            self.diagnostics.push(Diagnostic {
                file: self.path.to_path_buf(),
                line: anchor,
                kind: NAME,
                message: format!("{target} is {count} lines (limit {})", self.max_lines),
            });
        }
    }

    /// True when the 1-based `line` or the line immediately above it contains
    /// the `lint:allow(funclen)` escape hatch.
    fn is_suppressed(&self, line: usize) -> bool {
        let has_marker = |n: usize| {
            n >= 1
                && self
                    .lines
                    .get(n - 1)
                    .is_some_and(|l| l.contains("lint:allow(funclen)"))
        };
        has_marker(line) || has_marker(line.saturating_sub(1))
    }
}

impl<'ast> Visit<'ast> for FnVisitor<'_> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if is_test_cfg(&node.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if is_test_cfg(&node.attrs) {
            return;
        }
        syn::visit::visit_item_impl(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        if is_test_cfg(&node.attrs) {
            return;
        }
        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if is_test_cfg(&node.attrs) || is_test_fn(&node.attrs) {
            return;
        }
        let line = node.sig.fn_token.span.start().line;
        self.check_block(&node.block, line, Some(node.sig.ident.to_string()));
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if is_test_cfg(&node.attrs) || is_test_fn(&node.attrs) {
            return;
        }
        let line = node.sig.fn_token.span.start().line;
        self.check_block(&node.block, line, Some(node.sig.ident.to_string()));
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if is_test_cfg(&node.attrs) || is_test_fn(&node.attrs) {
            return;
        }
        if let Some(block) = &node.default {
            let line = node.sig.fn_token.span.start().line;
            self.check_block(block, line, Some(node.sig.ident.to_string()));
        }
        syn::visit::visit_trait_item_fn(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        if let syn::Expr::Block(body) = &*node.body {
            let line = node.or1_token.span.start().line;
            self.check_block(&body.block, line, None);
        }
        syn::visit::visit_expr_closure(self, node);
    }
}

/// Count the non-blank, non-comment source lines strictly between the block's
/// braces (the brace lines themselves are excluded).
fn count_body_lines(lines: &[&str], block: &syn::Block) -> usize {
    let open = block.brace_token.span.open().start().line;
    let close = block.brace_token.span.close().start().line;
    if close <= open + 1 {
        return 0;
    }
    lines
        .get(open..close - 1)
        .unwrap_or(&[])
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .count()
}

/// True for `#[cfg(test)]`, `#[cfg(feature = "test")]`, `#[cfg(any(test, ...))]`
/// and similar test-gating cfg attributes.
fn is_test_cfg(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if let syn::Meta::List(ref list) = attr.meta {
            return list.path.is_ident("cfg") && list.tokens.to_string().contains("test");
        }
        false
    })
}

/// True for `#[test]`, `#[tokio::test]`, and `#[tokio::test(...)]` functions.
fn is_test_fn(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| match &attr.meta {
        syn::Meta::Path(path) => path.segments.last().is_some_and(|s| s.ident == "test"),
        syn::Meta::List(list) => list.path.segments.last().is_some_and(|s| s.ident == "test"),
        _ => false,
    })
}

#[cfg(test)]
mod tests;
