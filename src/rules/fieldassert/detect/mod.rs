// standard crates
use std::collections::HashMap;

// internal crates
use crate::rules::{fieldassert::extract::root_receiver, Diagnostic, Input};

// external crates
use proc_macro2::{TokenStream, TokenTree};

const NAME: &str = "field-by-field-assert";

/// Scan a source file for test functions that have too many field-by-field
/// `assert_eq!` calls on the same receiver variable.
pub fn check(input: &Input) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    collect_from_items(&input.ast.items, input, &mut diagnostics);
    diagnostics
}

fn collect_from_items(items: &[syn::Item], input: &Input, diagnostics: &mut Vec<Diagnostic>) {
    for item in items {
        match item {
            syn::Item::Fn(item_fn) if is_test_fn(item_fn) => {
                check_test_fn(item_fn, input, diagnostics);
            }
            syn::Item::Mod(item_mod) => {
                if let Some((_, ref items)) = item_mod.content {
                    collect_from_items(items, input, diagnostics);
                }
            }
            _ => {}
        }
    }
}

fn is_test_fn(f: &syn::ItemFn) -> bool {
    f.attrs.iter().any(|attr| {
        if let syn::Meta::Path(ref path) = attr.meta {
            // #[test]
            if path.segments.last().is_some_and(|s| s.ident == "test") {
                return true;
            }
        }
        if let syn::Meta::List(ref list) = attr.meta {
            // #[tokio::test] or #[tokio::test(...)]
            if list.path.segments.last().is_some_and(|s| s.ident == "test") {
                return true;
            }
        }
        false
    })
}

fn check_test_fn(item_fn: &syn::ItemFn, input: &Input, diagnostics: &mut Vec<Diagnostic>) {
    // Use span to find the line range of the function in the source.
    let open_span = item_fn.block.brace_token.span.open();
    let close_span = item_fn.block.brace_token.span.close();
    let start_line = open_span.start().line;
    let end_line = close_span.end().line;

    // Extract the source lines for this function and check for escape hatch.
    let lines: Vec<&str> = input.source.lines().collect();
    let fn_start = start_line.saturating_sub(1); // 0-indexed
    let fn_end = end_line.min(lines.len());
    for line in &lines[fn_start..fn_end] {
        if line.contains("lint:allow(field-by-field-assert)") {
            return;
        }
    }

    // Collect assert_eq! calls with field access receivers.
    let mut records: Vec<(String, usize)> = Vec::new(); // (receiver, line)

    collect_assert_eqs_from_stmts(&item_fn.block.stmts, &mut records);

    // Group by receiver.
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (receiver, line) in records {
        groups.entry(receiver).or_default().push(line);
    }

    for (receiver, mut line_numbers) in groups {
        if line_numbers.len() >= input.limits.field_assert {
            line_numbers.sort();
            diagnostics.push(Diagnostic {
                file: input.path.to_path_buf(),
                line: line_numbers[0],
                kind: NAME,
                message: format!(
                    "{} assert_eq! calls on fields of `{receiver}` \u{2014} \
                     consider constructing an expected struct",
                    line_numbers.len()
                ),
            });
        }
    }
}

fn collect_assert_eqs_from_stmts(stmts: &[syn::Stmt], records: &mut Vec<(String, usize)>) {
    for stmt in stmts {
        collect_assert_eqs_from_stmt(stmt, records);
    }
}

fn collect_assert_eqs_from_stmt(stmt: &syn::Stmt, records: &mut Vec<(String, usize)>) {
    match stmt {
        syn::Stmt::Expr(expr, _) => collect_assert_eqs_from_expr(expr, records),
        syn::Stmt::Local(local) => {
            if let Some(init) = &local.init {
                collect_assert_eqs_from_expr(&init.expr, records);
            }
        }
        syn::Stmt::Item(_) => {}
        syn::Stmt::Macro(stmt_macro) => {
            check_macro(&stmt_macro.mac, records);
        }
    }
}

fn collect_assert_eqs_from_expr(expr: &syn::Expr, records: &mut Vec<(String, usize)>) {
    match expr {
        syn::Expr::Block(block) => {
            collect_assert_eqs_from_stmts(&block.block.stmts, records);
        }
        syn::Expr::If(expr_if) => {
            collect_assert_eqs_from_stmts(&expr_if.then_branch.stmts, records);
            if let Some((_, else_branch)) = &expr_if.else_branch {
                collect_assert_eqs_from_expr(else_branch, records);
            }
        }
        syn::Expr::Match(expr_match) => {
            for arm in &expr_match.arms {
                collect_assert_eqs_from_expr(&arm.body, records);
            }
        }
        syn::Expr::ForLoop(for_loop) => {
            collect_assert_eqs_from_stmts(&for_loop.body.stmts, records);
        }
        syn::Expr::While(while_loop) => {
            collect_assert_eqs_from_stmts(&while_loop.body.stmts, records);
        }
        syn::Expr::Loop(loop_expr) => {
            collect_assert_eqs_from_stmts(&loop_expr.body.stmts, records);
        }
        syn::Expr::Unsafe(unsafe_expr) => {
            collect_assert_eqs_from_stmts(&unsafe_expr.block.stmts, records);
        }
        syn::Expr::Macro(expr_macro) => {
            check_macro(&expr_macro.mac, records);
        }
        _ => {}
    }
}

fn check_macro(mac: &syn::Macro, records: &mut Vec<(String, usize)>) {
    let is_assert_eq = mac
        .path
        .segments
        .last()
        .is_some_and(|s| s.ident == "assert_eq");

    if !is_assert_eq {
        return;
    }

    if let Some(first_arg) = parse_first_arg(&mac.tokens) {
        if let Some(receiver) = root_receiver(&first_arg) {
            let line = mac.path.segments[0].ident.span().start().line;
            records.push((receiver, line));
        }
    }
}

/// Parse the first argument from an `assert_eq!` macro's token stream.
/// Splits on the first top-level comma (not inside any bracket/paren/brace group).
fn parse_first_arg(tokens: &TokenStream) -> Option<syn::Expr> {
    let mut first_arg_tokens = TokenStream::new();
    for tt in tokens.clone() {
        match &tt {
            TokenTree::Punct(p) if p.as_char() == ',' => {
                break;
            }
            _ => {
                first_arg_tokens.extend(std::iter::once(tt));
            }
        }
    }

    // Try to find the top-level comma more carefully: the simple approach above
    // works because proc_macro2 already groups delimited tokens into Group nodes.
    // A comma inside parens/brackets/braces is inside a Group and won't appear
    // as a top-level Punct. So the simple iteration is correct.

    syn::parse2::<syn::Expr>(first_arg_tokens).ok()
}

#[cfg(test)]
mod tests;
