// standard crates
use std::collections::HashSet;
use std::path::Path;

// internal crates
use crate::rules::{is_allowed_on_line_or_previous, is_code_line, Diagnostic, Input};

// external crates
use proc_macro2::{Delimiter, Group, TokenTree};
use syn::spanned::Spanned;
use syn::visit::Visit;

const NAME: &str = "funclen";

/// Scan a source file for functions and closures whose bodies exceed
/// `limits.funclen` non-blank, non-comment lines. A `limits.funclen` of 0 disables the
/// check.
pub fn check(input: &Input) -> Vec<Diagnostic> {
    if input.limits.funclen == 0 {
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

struct FnVisitor<'a> {
    path: &'a Path,
    lines: Vec<&'a str>,
    max_lines: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'ast> Visit<'ast> for FnVisitor<'_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let line = node.sig.fn_token.span.start().line;
        self.check_block(&node.block, line, Some(node.sig.ident.to_string()));
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let line = node.sig.fn_token.span.start().line;
        self.check_block(&node.block, line, Some(node.sig.ident.to_string()));
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if let Some(block) = &node.default {
            let line = node.sig.fn_token.span.start().line;
            self.check_block(block, line, Some(node.sig.ident.to_string()));
        }
        syn::visit::visit_trait_item_fn(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        if let syn::Expr::Block(body) = &*node.body {
            let line = node.inputs_begin.span.start().line;
            self.check_block(&body.block, line, None);
        }
        syn::visit::visit_expr_closure(self, node);
    }
}

impl FnVisitor<'_> {
    fn check_block(&mut self, block: &syn::Block, anchor: usize, name: Option<String>) {
        if is_allowed_on_line_or_previous(&self.lines, anchor, NAME) {
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
}

/// Count the non-blank, non-comment source lines strictly between the block's
/// braces (the brace lines themselves are excluded), skipping `literal_tail_lines`.
fn count_body_lines(lines: &[&str], block: &syn::Block) -> usize {
    let open = block.brace_token.span.open().start().line;
    let close = block.brace_token.span.close().start().line;
    let skip = literal_tail_lines(block);
    (open + 1..close)
        .filter(|line| !skip.contains(line))
        .filter(|line| lines.get(line - 1).is_some_and(|text| is_code_line(text)))
        .count()
}

fn literal_tail_lines(block: &syn::Block) -> HashSet<usize> {
    let mut finder = LiteralFinder::default();
    finder.visit_block(block);
    let mut skip = HashSet::new();
    for (first, last) in &finder.literals {
        skip.extend(first + 1..=*last);
    }
    for (_, last) in &finder.literals {
        if finder.code_starts.contains(last) {
            skip.remove(last);
        }
    }
    skip
}

#[derive(Default)]
struct LiteralFinder {
    literals: Vec<(usize, usize)>,
    code_starts: Vec<usize>,
    has_block: bool,
}

impl<'ast> Visit<'ast> for LiteralFinder {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        let outer_has_block = std::mem::take(&mut self.has_block);
        let literal_mark = self.literals.len();
        let span = node.span();
        self.code_starts.push(span.start().line);
        let start_mark = self.code_starts.len();
        syn::visit::visit_expr(self, node);
        if is_composite_literal(node) && !self.has_block {
            self.literals.truncate(literal_mark);
            self.code_starts.truncate(start_mark);
            self.literals.push((span.start().line, span.end().line));
        }
        self.has_block |= outer_has_block;
    }

    fn visit_block(&mut self, node: &'ast syn::Block) {
        self.has_block = true;
        syn::visit::visit_block(self, node);
    }

    fn visit_arm(&mut self, node: &'ast syn::Arm) {
        self.has_block = true;
        syn::visit::visit_arm(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.has_block = true;
        syn::visit::visit_expr_closure(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        syn::visit::visit_macro(self, node);
        if let Some(args) = macro_args(node) {
            syn::visit::visit_expr(self, &args);
        }
    }

    fn visit_ident(&mut self, node: &'ast syn::Ident) {
        self.code_starts.push(node.span().start().line);
    }
}

fn is_composite_literal(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Struct(_) | syn::Expr::Array(_) | syn::Expr::Repeat(_) => true,
        syn::Expr::Macro(node) => node.mac.path.is_ident("vec") && macro_args(&node.mac).is_some(),
        _ => false,
    }
}

pub fn macro_args(mac: &syn::Macro) -> Option<syn::Expr> {
    let group = Group::new(Delimiter::Bracket, mac.tokens.clone());
    syn::parse2(TokenTree::Group(group).into()).ok()
}

#[cfg(test)]
mod tests;
