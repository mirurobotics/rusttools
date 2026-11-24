// standard crates
use std::collections::{HashMap, HashSet};

// internal crates
use crate::rules::{funclen::macro_args, is_allowed_on_line_or_previous, Diagnostic, Input};

// external crates
use syn::visit::Visit;

const NAME: &str = "funcorder";

struct Function {
    name: String,
    line: usize,
    callees: HashSet<String>,
}

struct References<'a> {
    owner: Option<&'a str>,
    names: HashSet<String>,
    locals: HashSet<String>,
}

struct Graph<'a> {
    functions: &'a [Function],
    index: HashMap<&'a str, &'a Function>,
}

pub fn check(input: &Input) -> Vec<Diagnostic> {
    let lines: Vec<&str> = input.source.lines().collect();
    scopes(&input.ast.items)
        .into_iter()
        .flat_map(|items| Graph::new(&functions(items, &lines)).misplaced())
        .map(|(line, message)| Diagnostic {
            file: input.path.to_path_buf(),
            line,
            kind: NAME,
            message,
        })
        .collect()
}

fn scopes(items: &[syn::Item]) -> Vec<&[syn::Item]> {
    let nested = items.iter().filter_map(module_items).flat_map(scopes);
    std::iter::once(items).chain(nested).collect()
}

fn module_items(item: &syn::Item) -> Option<&[syn::Item]> {
    let syn::Item::Mod(module) = item else {
        return None;
    };
    let (_, items) = module.content.as_ref()?;
    Some(items.as_slice())
}

fn functions(items: &[syn::Item], lines: &[&str]) -> Vec<Function> {
    let mut functions = Vec::new();
    for item in items {
        match item {
            syn::Item::Fn(function) => {
                functions.push(Function::new(None, &function.sig, &function.block));
            }
            syn::Item::Impl(item) => functions.extend(methods(item)),
            _ => {}
        }
    }
    functions.retain(|function| !is_allowed_on_line_or_previous(lines, function.line, NAME));
    functions
}

fn methods(item: &syn::ItemImpl) -> Vec<Function> {
    let Some(name) = type_name(&item.self_ty) else {
        return Vec::new();
    };
    let owner = Some(name.as_str());
    let mut methods = Vec::new();
    for impl_item in &item.items {
        if let syn::ImplItem::Fn(method) = impl_item {
            methods.push(Function::new(owner, &method.sig, &method.block));
        }
    }
    methods
}

fn type_name(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(syn::TypePath { path, .. }) = ty else {
        return None;
    };
    path.segments.last().map(|last| last.ident.to_string())
}

impl Function {
    fn new(owner: Option<&str>, sig: &syn::Signature, block: &syn::Block) -> Self {
        let mut references = References {
            owner,
            names: HashSet::new(),
            locals: HashSet::new(),
        };
        references.visit_signature(sig);
        references.visit_block(block);
        let name = match owner {
            Some(owner) => format!("{owner}::{}", sig.ident),
            None => sig.ident.to_string(),
        };
        Self {
            name,
            line: sig.fn_token.span.start().line,
            callees: &references.names - &references.locals,
        }
    }
}

impl<'ast> Visit<'ast> for References<'_> {
    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        self.names.extend(referenced_name(self.owner, &node.path));
        syn::visit::visit_expr_path(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if let Some(owner) = self.owner.filter(|_| is_self(&node.receiver)) {
            self.names.insert(format!("{owner}::{}", node.method));
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        self.locals.insert(node.ident.to_string());
        syn::visit::visit_pat_ident(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if let Some(args) = macro_args(node) {
            syn::visit::visit_expr(self, &args);
        }
    }
}

fn referenced_name(owner: Option<&str>, path: &syn::Path) -> Option<String> {
    let mut segments = path.segments.iter().map(|segment| &segment.ident);
    match (segments.next(), segments.next(), segments.next()) {
        (Some(name), None, _) => Some(name.to_string()),
        (Some(first), Some(name), None) if first == "Self" => {
            owner.map(|owner| format!("{owner}::{name}"))
        }
        _ => None,
    }
}

fn is_self(expr: &syn::Expr) -> bool {
    matches!(expr, syn::Expr::Path(path) if path.path.is_ident("self"))
}

impl<'a> Graph<'a> {
    fn new(functions: &'a [Function]) -> Self {
        let index = functions
            .iter()
            .map(|function| (function.name.as_str(), function))
            .collect();
        Self { functions, index }
    }

    fn misplaced(&self) -> Vec<(usize, String)> {
        self.functions
            .iter()
            .filter_map(|callee| {
                let caller = self.last_caller(callee)?;
                let message = format!(
                    "function `{}` should be below its last caller `{}`",
                    callee.name, caller.name
                );
                (caller.line > callee.line).then_some((callee.line, message))
            })
            .collect()
    }

    fn last_caller(&self, callee: &Function) -> Option<&'a Function> {
        self.functions
            .iter()
            .filter(|caller| caller.callees.contains(&callee.name))
            .filter(|caller| !self.reaches(&callee.name, &caller.name, &mut HashSet::new()))
            .max_by_key(|caller| caller.line)
    }

    fn reaches(&self, from: &str, to: &str, seen: &mut HashSet<&'a str>) -> bool {
        if from == to {
            return true;
        }
        let Some(&function) = self.index.get(from) else {
            return false;
        };
        if !seen.insert(function.name.as_str()) {
            return false;
        }
        let mut callees = function.callees.iter();
        callees.any(|next| self.reaches(next, to, seen))
    }
}

#[cfg(test)]
mod tests;
