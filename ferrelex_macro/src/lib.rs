// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use decision_tree::DecisionTree;
use ferrelex_core::cset::CSet;
use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use rustc_hash::FxHashMap;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicI32, Ordering},
    },
};
use syn::{
    Arm, Attribute, Expr, ExprMatch, ExprPath, File, FnArg, Ident, ItemFn, Pat, PatType, Type,
    TypePath, parse_macro_input, visit_mut::VisitMut,
};

use crate::regex::{Regex, case_fold_regex, compile, regex_of_expr, regex_of_pattern};

mod decision_tree;
mod regex;

pub(crate) type Env = FxHashMap<String, Regex>;

static PARTITIONS: LazyLock<Mutex<FxHashMap<Vec<CSet>, String>>> =
    LazyLock::new(|| Mutex::new(FxHashMap::default()));

static PARTITION_COUNTER: LazyLock<AtomicI32> = LazyLock::new(|| AtomicI32::new(0));

static TABLES: LazyLock<Mutex<FxHashMap<Vec<u8>, String>>> =
    LazyLock::new(|| Mutex::new(FxHashMap::default()));

static TABLE_COUNTER: LazyLock<AtomicI32> = LazyLock::new(|| AtomicI32::new(0));

fn best_final(finals: &[bool]) -> Option<usize> {
    finals.iter().position(|&f| f)
}

fn set_state(auto: &[(Vec<(CSet, i32)>, Vec<bool>)], state: i32) -> TokenStream {
    let (trans, final_) = &auto[state as usize];
    if trans.is_empty() {
        let i = best_final(final_).unwrap() as i32;
        quote! {break #i;}.into()
    } else {
        // appfun(&state_fun(state), vec![lexbuf.into_token_stream().into()])
        quote! { state = #state; }.into()
    }
}

fn get_partitions() -> Vec<(String, Vec<CSet>)> {
    let partitions = PARTITIONS.lock().expect("partition lock not poisoned");
    partitions
        .iter()
        .map(|(k, v)| (v.clone(), k.clone()))
        .collect()
}

fn partition_name(p: Vec<CSet>) -> Ident {
    let mut partitions = PARTITIONS.lock().expect("partition lock not poisoned");
    match partitions.get(&p) {
        None => {
            let mut c = PARTITION_COUNTER.fetch_add(1, Ordering::Relaxed);
            c += 1;
            let name = format!("__ferrelex_partition_{c}");
            partitions.insert(p.clone(), name.clone());
            format_ident!("{name}")
        }
        Some(n) => {
            format_ident!("{n}")
        }
    }
}

fn partition(name: &str, p: &[CSet]) -> proc_macro2::TokenStream {
    let body = DecisionTree::decision_table(p)
        .simplify_decision_tree()
        .gen_tokens();
    let name = format_ident!("{name}");
    quote! {
        #[inline]
        fn #name(c: i32) -> i32 {
            #body
        }
    }
    .into()
}

pub(crate) fn table_name(t: &[u8]) -> Ident {
    let mut tables = TABLES.lock().expect("table lock not poisoned");
    match tables.get(t) {
        None => {
            let mut c = TABLE_COUNTER.fetch_add(1, Ordering::Relaxed);
            c += 1;
            let name = format!("__ferrelex_table_{c}");
            tables.insert(t.to_vec(), name.clone());
            format_ident!("{name}")
        }
        Some(n) => {
            format_ident!("{n}")
        }
    }
}

fn table(name: &str, t: &[u8]) -> proc_macro2::TokenStream {
    let name = format_ident!("{name}");
    quote! { static #name: &[u8] = &[#(#t,)*]; }
}

fn get_tables() -> Vec<(String, Vec<u8>)> {
    let tables = TABLES.lock().expect("table lock not poisoned");
    tables.iter().map(|(k, v)| (v.clone(), k.clone())).collect()
}

fn gen_state_arm(
    lexbuf: &Ident,
    auto: &[(Vec<(CSet, i32)>, Vec<bool>)],
    i: i32,
    elt: &(Vec<(CSet, i32)>, Vec<bool>),
    track_lines: bool,
) -> TokenStream {
    let (trans, final_) = elt;
    let partition: Vec<_> = trans.iter().map(|(c, _)| c.clone()).collect();
    let mut cases: Vec<_> = trans
        .iter()
        .enumerate()
        .map(|(idx, (_, j))| {
            let e: proc_macro2::TokenStream = set_state(auto, *j).into();
            let idx = idx as i32;
            quote! { #idx => { #e } }
        })
        .collect();
    cases.push(quote! { _ => break #lexbuf.backtrack() });

    let name = partition_name(partition);

    let body = quote! {
        {
            let __ferrelex_c = #lexbuf.next_int(#track_lines);
            // Guard: -2 signals an invalid UTF-8 byte. Skip the partition function
            // (which may panic on values below -1) and go straight to backtrack.
            if __ferrelex_c < -1 {
                break #lexbuf.backtrack()
            }
            match #name(__ferrelex_c) {
                #(#cases),*
            }
        }
    };

    let body = match best_final(&final_) {
        None => body,
        Some(_) if trans.is_empty() => body,
        Some(fi) => {
            let fi = fi as i32;
            quote! {
                #lexbuf.mark(#fi);
                #body
            }
        }
    };

    quote! { #i => { #body }}.into()
}

fn gen_definition(
    lexbuf: &Ident,
    // None = skip action (restart DFA without returning)
    l: Vec<(Regex, Option<TokenStream>)>,
    error: TokenStream,
    track_lines: bool,
    case_insensitive: bool,
) -> TokenStream {
    let has_skip = l.iter().any(|(_, body)| body.is_none());

    let compiled_regex = {
        let fst: Vec<_> = l
            .iter()
            .map(|(r, _)| {
                if case_insensitive {
                    case_fold_regex(r.clone())
                } else {
                    r.clone()
                }
            })
            .collect();
        compile(fst.as_slice())
    };

    let mut cases: Vec<proc_macro2::TokenStream> = l
        .iter()
        .enumerate()
        .map(|(i, (_, e))| {
            let i_i32 = i as i32;
            match e {
                Some(e) => {
                    let e: proc_macro2::TokenStream = e.clone().into();
                    if has_skip {
                        quote! { #i_i32 => break '__ferrelex_skip { #e } }
                    } else {
                        quote! { #i_i32 => { #e } }
                    }
                }
                None => quote! { #i_i32 => continue '__ferrelex_skip },
            }
        })
        .collect();

    let error: proc_macro2::TokenStream = error.into();
    cases.push(if has_skip {
        quote! {
            _ => {
                #[allow(unused_variables)]
                let #lexbuf = ::ferrelex::__private::WildcardLexBuf(#lexbuf);
                break '__ferrelex_skip { #error }
            }
        }
        .into()
    } else {
        quote! {
            _ => {
                #[allow(unused_variables)]
                let #lexbuf = ::ferrelex::__private::WildcardLexBuf(#lexbuf);
                #error
            }
        }
        .into()
    });

    let arms: Vec<proc_macro2::TokenStream> = compiled_regex
        .iter()
        .enumerate()
        .filter(|(_, (trans, _))| !trans.is_empty())
        .map(|(i, elt)| gen_state_arm(lexbuf, &compiled_regex, i as i32, elt, track_lines).into())
        .collect();

    let inner = quote! {
        #lexbuf.start();
        let __ferrelex_result = {
            let mut state = 0i32;
            loop {
                match state {
                    #(#arms,)*
                    _ => ::core::unreachable!(),
                }
            }
        };
        match __ferrelex_result {
            #(#cases),*
        }
    };

    if has_skip {
        quote! { '__ferrelex_skip: loop { #inner } }.into()
    } else {
        inner.into()
    }
}

struct RecursionFinder<'ast> {
    fn_name: &'ast str,
    found: Option<&'ast Expr>,
}

impl<'ast> syn::visit::Visit<'ast> for RecursionFinder<'ast> {
    fn visit_expr(&mut self, node: &'ast Expr) {
        if self.found.is_some() {
            return;
        }
        if let Expr::Call(call) = node {
            if matches!(call.func.as_ref(), Expr::Path(p) if p.path.is_ident(self.fn_name)) {
                self.found = Some(node);
                return;
            }
        }
        syn::visit::visit_expr(self, node);
    }
}

fn find_recursive_call<'a>(expr: &'a Expr, fn_name: &'a str) -> Option<&'a Expr> {
    let mut finder = RecursionFinder {
        fn_name,
        found: None,
    };
    syn::visit::Visit::visit_expr(&mut finder, expr);
    finder.found
}

fn is_skip_arm(arm: &Arm) -> bool {
    arm.attrs.iter().any(|a| a.path().is_ident("skip"))
}

struct LexerTransformer {
    env: Rc<RefCell<Env>>,
    args: Vec<FnArg>,
    fn_name: String,
}

impl LexerTransformer {
    fn process_lexer_match(
        &self,
        expr_match: &ExprMatch,
        attr: &Attribute,
    ) -> proc_macro2::TokenStream {
        let ExprMatch { expr, arms, .. } = expr_match;

        let opts = match parse_lexer_attr(attr) {
            Ok(v) => v,
            Err(e) => return e.to_compile_error(),
        };
        let track_lines = !opts.no_line_tracking;
        let allow_recursion = opts.allow_recursion;
        let case_insensitive = opts.case_insensitive;

        let lexbuf = match expr.as_ref() {
            Expr::Path(ExprPath {
                qself: None, path, ..
            }) if path.segments.len() == 1 => path.segments[0].ident.clone(),
            _ => {
                return syn::Error::new_spanned(
                    expr,
                    "expecting only an identifier to match against regexes.",
                )
                .to_compile_error();
            }
        };

        if !allow_recursion {
            for arm in &arms[..arms.len().saturating_sub(1)] {
                if is_skip_arm(arm) {
                    continue;
                }
                if let Some(call) = find_recursive_call(&arm.body, &self.fn_name) {
                    return syn::Error::new_spanned(
                        call,
                        format!(
                            "recursive call to `{}` in a `#[lexer]` match arm will \
                            stack-overflow on long inputs; use \
                            `loop {{ #[lexer] match ... }}` instead, or suppress this \
                            error with `#[lexer(allow_recursion)]`",
                            self.fn_name
                        ),
                    )
                    .to_compile_error();
                }
            }
        }

        let error: TokenStream = match arms.last() {
            Some(Arm {
                pat: Pat::Wild(_),
                body,
                guard: None,
                ..
            }) => body.to_token_stream().into(),
            _ => {
                return syn::Error::new_spanned(
                    expr,
                    "expecting a wildcard for error handling as last match arm.",
                )
                .to_compile_error();
            }
        };

        let cases: Result<Vec<(Regex, Option<TokenStream>)>, _> = arms[..arms.len() - 1]
            .iter()
            .map(|arm| match arm {
                Arm { guard: Some(_), .. } => {
                    Err(syn::Error::new_spanned(arm, "guards are not supported"))
                }
                Arm { pat, body, .. } => {
                    let action = if is_skip_arm(arm) {
                        None
                    } else {
                        Some(body.to_token_stream().into())
                    };
                    Ok((regex_of_pattern(self.env.clone(), pat.clone())?, action))
                }
            })
            .collect();

        let cases = match cases {
            Ok(c) => c,
            Err(e) => return e.to_compile_error(),
        };

        let lexbuf_found = self.args.iter().any(|fn_arg| match fn_arg {
            FnArg::Typed(PatType { pat, .. }) => {
                matches!(pat.as_ref(), Pat::Ident(p) if p.ident == lexbuf)
            }
            _ => false,
        });

        if !lexbuf_found {
            return syn::Error::new_spanned(
                &lexbuf,
                format!("could not find `{}` in fn parameters.", lexbuf),
            )
            .to_compile_error();
        }

        gen_definition(&lexbuf, cases, error, track_lines, case_insensitive).into()
    }
}

impl syn::visit_mut::VisitMut for LexerTransformer {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // Post-order: recurse into all children first, then transform this node.
        // This means nested #[lexer] matches inside arm bodies are expanded before
        // we read the arm bodies to build the outer DFA.
        syn::visit_mut::visit_expr_mut(self, expr);

        if let Expr::Match(expr_match) = expr {
            if let Some(idx) = expr_match
                .attrs
                .iter()
                .position(|a| a.path().is_ident("lexer"))
            {
                let attr = expr_match.attrs.remove(idx);
                let ts = self.process_lexer_match(expr_match, &attr);
                *expr = Expr::Verbatim(ts);
            }
        }
    }

    fn visit_item_fn_mut(&mut self, _: &mut ItemFn) {
        // Do not recurse into nested fn items — matches the old transform_stmts
        // behaviour of treating Stmt::Item as a verbatim pass-through.
    }
}

#[derive(Default)]
struct LexerOptions {
    no_line_tracking: bool,
    allow_recursion: bool,
    case_insensitive: bool,
}

fn parse_lexer_attr(attr: &Attribute) -> Result<LexerOptions, syn::Error> {
    let mut opts = LexerOptions::default();
    match &attr.meta {
        // #[lexer]
        syn::Meta::Path(_) => {}
        // #[lexer(...)]
        syn::Meta::List(_) => {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("no_line_tracking") {
                    opts.no_line_tracking = true;
                    Ok(())
                } else if meta.path.is_ident("allow_recursion") {
                    opts.allow_recursion = true;
                    Ok(())
                } else if meta.path.is_ident("case_insensitive") {
                    opts.case_insensitive = true;
                    Ok(())
                } else {
                    Err(meta.error("unknown lexer option"))
                }
            })?;
        }
        syn::Meta::NameValue(_) => {
            return Err(syn::Error::new_spanned(
                attr,
                "Expected `#[lexer]` or `#[lexer(<options>)]`",
            ));
        }
    }
    Ok(opts)
}

#[proc_macro]
pub fn lex(input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as File);
    let env = Rc::new(RefCell::new(regex::builtin_regex()));
    let mut res = quote! {};
    let regex_typ: TypePath = syn::parse_quote! {Regex};
    for item in f.items {
        match item {
            syn::Item::Const(item_const) => match *item_const.ty {
                Type::Path(path) if path == regex_typ => {
                    let name = item_const.ident.to_string();
                    let regex = match regex_of_expr(env.clone(), *item_const.expr) {
                        Ok(v) => v,
                        Err(e) => return e.to_compile_error().into(),
                    };
                    env.borrow_mut().insert(name, regex);
                }
                _ => {
                    return syn::Error::new_spanned(
                        &item_const,
                        "expecting only `Regex` typed const.",
                    )
                    .to_compile_error()
                    .into();
                }
            },
            syn::Item::Fn(mut item_fn) => {
                let args: Vec<FnArg> = item_fn.sig.inputs.iter().cloned().collect();
                let fn_name = item_fn.sig.ident.to_string();
                let mut transformer = LexerTransformer {
                    env: env.clone(),
                    args,
                    fn_name,
                };
                transformer.visit_block_mut(&mut item_fn.block);
                res.extend(item_fn.into_token_stream());
            }
            other => {
                return syn::Error::new_spanned(
                    &other,
                    "expecting only `const` and `fn` items in the `lex` macro",
                )
                .to_compile_error()
                .into();
            }
        }
    }
    let partitions: Vec<_> = get_partitions()
        .iter()
        .map(|(name, p)| partition(name, p))
        .collect();
    let tables: Vec<_> = get_tables()
        .iter()
        .map(|(name, t)| table(name, t))
        .collect();
    quote! {
        #(#tables)*
        #(#partitions)*
        #res
    }
    .into()
}
