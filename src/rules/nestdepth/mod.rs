// standard crates
use std::path::Path;

// internal crates
use crate::rules::{is_allowed_on_line_or_previous, Diagnostic, Input};

// external crates
use syn::visit::Visit;

const NAME: &str = "nestdepth";
const HINT: &str = "consider early returns or extracting a helper";

pub fn check(input: &Input) -> Vec<Diagnostic> {
    if input.limits.nestdepth == 0 {
        return Vec::new();
    }
    let mut visitor = NestVisitor {
        path: input.path,
        lines: input.source.lines().collect(),
        max_depth: input.limits.nestdepth,
        depth: 0,
        diagnostics: Vec::new(),
    };
    visitor.visit_file(input.ast);
    visitor.diagnostics
}

struct NestVisitor<'a> {
    path: &'a Path,
    lines: Vec<&'a str>,
    max_depth: usize,
    depth: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'ast> Visit<'ast> for NestVisitor<'_> {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        self.visit_at_depth_zero(|v| syn::visit::visit_item(v, node));
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.visit_at_depth_zero(|v| syn::visit::visit_expr_closure(v, node));
    }

    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        let Some(line) = nesting_line(node) else {
            syn::visit::visit_expr(self, node);
            return;
        };
        self.depth += 1;
        if self.depth == self.max_depth + 1 {
            self.report(line);
        }
        syn::visit::visit_expr(self, node);
        self.depth -= 1;
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.visit_expr(&node.cond);
        self.visit_block(&node.then_branch);
        match node.else_branch.as_ref().map(|(_, branch)| &**branch) {
            Some(syn::Expr::If(next)) => self.visit_expr_if(next),
            Some(branch) => self.visit_expr(branch),
            None => {}
        }
    }
}

impl NestVisitor<'_> {
    fn visit_at_depth_zero(&mut self, visit: impl FnOnce(&mut Self)) {
        let outer = std::mem::take(&mut self.depth);
        visit(self);
        self.depth = outer;
    }

    fn report(&mut self, line: usize) {
        if is_allowed_on_line_or_previous(&self.lines, line, NAME) {
            return;
        }
        self.diagnostics.push(Diagnostic {
            file: self.path.to_path_buf(),
            line,
            kind: NAME,
            message: format!(
                "nesting depth is {} (limit {}) — {HINT}",
                self.depth, self.max_depth
            ),
        });
    }
}

fn nesting_line(expr: &syn::Expr) -> Option<usize> {
    let span = match expr {
        syn::Expr::If(node) => node.if_token.span,
        syn::Expr::Match(node) => node.match_token.span,
        syn::Expr::ForLoop(node) => node.for_token.span,
        syn::Expr::While(node) => node.while_token.span,
        syn::Expr::Loop(node) => node.loop_token.span,
        _ => return None,
    };
    Some(span.start().line)
}

#[cfg(test)]
mod tests;
