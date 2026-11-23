// standard crates
use std::collections::HashSet;
use std::path::Path;

// internal crates
use crate::rules::{is_allowed_on_line_or_previous, is_code_line, Diagnostic, Input};

// external crates
use syn::parse::ParseStream;
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
            let line = node.or1_token.span.start().line;
            self.check_block(&body.block, line, None);
        }
        syn::visit::visit_expr_closure(self, node);
    }
}

/// Count the non-blank, non-comment source lines strictly between the block's
/// braces (the brace lines themselves are excluded), counting a multi-line
/// literal without nested blocks as one line.
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
    code_starts: HashSet<usize>,
}

impl<'ast> Visit<'ast> for LiteralFinder {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        let span = node.span();
        self.code_starts.insert(span.start().line);
        if is_collapsible_literal(node) {
            self.literals.push((span.start().line, span.end().line));
            return;
        }
        visit_vec_elements(self, node);
        syn::visit::visit_expr(self, node);
    }

    fn visit_ident(&mut self, node: &'ast syn::Ident) {
        self.code_starts.insert(node.span().start().line);
    }
}

fn is_collapsible_literal(expr: &syn::Expr) -> bool {
    if matches!(expr, syn::Expr::Struct(_) | syn::Expr::Array(_)) {
        return !has_block_expr(expr);
    }
    let Some(elements) = vec_elements(expr) else {
        return false;
    };
    !elements.iter().any(has_block_expr)
}

fn has_block_expr(expr: &syn::Expr) -> bool {
    let mut finder = BlockFinder::default();
    finder.visit_expr(expr);
    finder.found
}

#[derive(Default)]
struct BlockFinder {
    found: bool,
}

impl<'ast> Visit<'ast> for BlockFinder {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if is_block_bearing(node) {
            self.found = true;
            return;
        }
        visit_vec_elements(self, node);
        syn::visit::visit_expr(self, node);
    }
}

fn is_block_bearing(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Match(_)
            | syn::Expr::If(_)
            | syn::Expr::While(_)
            | syn::Expr::Loop(_)
            | syn::Expr::ForLoop(_)
            | syn::Expr::Block(_)
            | syn::Expr::Unsafe(_)
            | syn::Expr::Async(_)
            | syn::Expr::Closure(_)
    )
}

fn visit_vec_elements<V: for<'a> Visit<'a>>(visitor: &mut V, expr: &syn::Expr) {
    for element in vec_elements(expr).unwrap_or_default() {
        visitor.visit_expr(&element);
    }
}

fn vec_elements(expr: &syn::Expr) -> Option<Vec<syn::Expr>> {
    let syn::Expr::Macro(node) = expr else {
        return None;
    };
    if !node.mac.path.is_ident("vec") {
        return None;
    }
    node.mac.parse_body_with(parse_vec_body).ok()
}

fn parse_vec_body(input: ParseStream) -> syn::Result<Vec<syn::Expr>> {
    let mut elements = Vec::new();
    while !input.is_empty() {
        elements.push(input.parse()?);
        if !input.is_empty() && input.parse::<Option<syn::Token![;]>>()?.is_none() {
            input.parse::<syn::Token![,]>()?;
        }
    }
    Ok(elements)
}

#[cfg(test)]
mod tests;
